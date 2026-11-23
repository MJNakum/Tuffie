//! Policy checks with Rego (Open Policy Agent language), evaluated in-process
//! with `regorus`. No `opa` binary is needed.
//!
//! Convention (compatible with Conftest):
//! - The input is the plan JSON from `terraform show -json` (`input.resource_changes`, …).
//! - Rules named `deny` or `violation` are failures.
//! - Rules named `warn` are warnings.
//! - A rule value is a set of strings, or of objects with a `msg` field.
//! - Rego v1 syntax is preferred. Older v0 syntax is accepted as a fallback.
//!
//! The `http` builtins are not compiled in, so a policy cannot make network calls.
//! Each evaluation has a time limit.

use std::num::NonZeroU32;
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use crate::models::{Policy, PolicySet};
use crate::AppState;

const FAIL_RULES: [&str; 2] = ["deny", "violation"];
const WARN_RULES: [&str; 1] = ["warn"];
const TIME_LIMIT: Duration = Duration::from_secs(10);
const MAX_FILES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// `fail` or `warn`.
    pub level: String,
    /// The Rego package, for example `terraform.s3`.
    pub package: String,
    pub rule: String,
    pub message: String,
    /// Extra fields from an object result (everything except `msg`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetResult {
    pub set_id: String,
    pub set_name: String,
    pub enforcement: String,
    /// `passed`, `warned`, `failed` or `errored`.
    pub status: String,
    pub policy_count: usize,
    pub error: Option<String>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Overall: `passed`, `warned`, `failed` (a mandatory set failed) or `errored`.
    pub status: String,
    pub sets: Vec<SetResult>,
}

impl Report {
    /// True when the plan must not be applied.
    pub fn blocks_apply(&self) -> bool {
        self.sets.iter().any(|s| s.enforcement == "mandatory" && matches!(s.status.as_str(), "failed" | "errored"))
    }
}

/// Load the Rego sources of a set: from the database, or from `*.rego` files in a folder.
pub async fn load_sources(state: &AppState, set: &PolicySet) -> anyhow::Result<Vec<(String, String)>> {
    match set.source_kind.as_str() {
        "inline" => {
            let rows: Vec<Policy> = sqlx::query_as("SELECT * FROM policies WHERE set_id = ? ORDER BY name").bind(&set.id).fetch_all(&state.db).await?;
            Ok(rows.into_iter().map(|p| (p.name, p.rego)).collect())
        }
        "path" => {
            let dir = set.source_path.clone().context("policy set has no folder")?;
            tokio::task::spawn_blocking(move || read_folder(Path::new(&dir))).await?
        }
        other => Err(anyhow!("unknown policy source {other}")),
    }
}

fn read_folder(dir: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = std::fs::read_dir(&d).with_context(|| format!("cannot read policy folder {}", d.display()))?;
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if name.ends_with(".rego") && !name.ends_with("_test.rego") {
                let rel = p.strip_prefix(dir).unwrap_or(&p).display().to_string();
                out.push((rel, std::fs::read_to_string(&p)?));
                if out.len() > MAX_FILES {
                    return Err(anyhow!("more than {MAX_FILES} policy files"));
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Compile the sources. Used to validate a policy when it is saved.
pub fn compile(sources: &[(String, String)]) -> anyhow::Result<regorus::Engine> {
    let mut engine = regorus::Engine::new();
    engine.set_execution_timer_config(regorus::utils::limits::ExecutionTimerConfig {
        limit: TIME_LIMIT,
        check_interval: NonZeroU32::new(64).unwrap(),
    });
    for (name, rego) in sources {
        if let Err(v1_err) = engine.add_policy(name.clone(), rego.clone()) {
            // Fall back to Rego v0 syntax (`deny[msg] { … }`), common in older policies.
            engine.set_rego_v0(true);
            let r = engine.add_policy(name.clone(), rego.clone());
            engine.set_rego_v0(false);
            if r.is_err() {
                return Err(anyhow!("{name}: {v1_err}"));
            }
        }
    }
    Ok(engine)
}

/// Evaluate compiled policies against an input document.
pub fn evaluate(sources: &[(String, String)], input_json: &str) -> anyhow::Result<Vec<Finding>> {
    let mut engine = compile(sources)?;
    engine.set_input_json(input_json)?;
    let mut findings = vec![];
    for pkg in engine.get_packages()? {
        for (rules, level) in [(&FAIL_RULES[..], "fail"), (&WARN_RULES[..], "warn")] {
            for rule in rules {
                let path = format!("{pkg}.{rule}");
                let value = match engine.eval_rule(path) {
                    Ok(v) => v,
                    // The package does not define this rule.
                    Err(e) if e.to_string().contains("not a valid rule path") => continue,
                    Err(e) => return Err(anyhow!("{pkg}.{rule}: {e}")),
                };
                if value == regorus::Value::Undefined {
                    continue;
                }
                let json: serde_json::Value = serde_json::from_str(&value.to_json_str()?)?;
                let items = match json {
                    serde_json::Value::Array(a) => a,
                    serde_json::Value::Bool(false) | serde_json::Value::Null => vec![],
                    other => vec![other],
                };
                for item in items {
                    let (message, details) = match item {
                        serde_json::Value::String(s) => (s, None),
                        serde_json::Value::Object(mut o) => {
                            let msg = o.remove("msg").and_then(|m| m.as_str().map(String::from)).unwrap_or_else(|| "(no message)".into());
                            (msg, if o.is_empty() { None } else { Some(serde_json::Value::Object(o)) })
                        }
                        serde_json::Value::Bool(true) => (format!("{rule} is true"), None),
                        other => (other.to_string(), None),
                    };
                    findings.push(Finding {
                        level: level.into(),
                        package: pkg.trim_start_matches("data.").to_string(),
                        rule: rule.to_string(),
                        message,
                        details,
                    });
                }
            }
        }
    }
    Ok(findings)
}

fn set_status(findings: &[Finding]) -> &'static str {
    if findings.iter().any(|f| f.level == "fail") {
        "failed"
    } else if findings.is_empty() {
        "passed"
    } else {
        "warned"
    }
}

/// Run every policy set that applies to a workspace. Returns `None` when
/// the workspace has no policy sets.
pub async fn check(state: &AppState, ws_id: &str, plan_json: &[u8]) -> anyhow::Result<Option<Report>> {
    let ids = crate::api::assign::sets_for_workspace(state, crate::api::assign::Kind::Policy, ws_id).await?;
    if ids.is_empty() {
        return Ok(None);
    }
    let input = String::from_utf8_lossy(plan_json).into_owned();
    let mut sets = vec![];
    for id in ids {
        let set: PolicySet = sqlx::query_as("SELECT * FROM policy_sets WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
        sets.push(run_set(state, &set, &input).await);
    }
    let status = if sets.iter().any(|s| s.status == "errored") {
        "errored"
    } else if sets.iter().any(|s| s.status == "failed" && s.enforcement == "mandatory") {
        "failed"
    } else if sets.iter().any(|s| s.status != "passed") {
        "warned"
    } else {
        "passed"
    };
    Ok(Some(Report { status: status.into(), sets }))
}

pub async fn run_set(state: &AppState, set: &PolicySet, input: &str) -> SetResult {
    let mut result = SetResult {
        set_id: set.id.clone(),
        set_name: set.name.clone(),
        enforcement: set.enforcement.clone(),
        status: "passed".into(),
        policy_count: 0,
        error: None,
        findings: vec![],
    };
    let sources = match load_sources(state, set).await {
        Ok(s) => s,
        Err(e) => {
            result.status = "errored".into();
            result.error = Some(format!("{e:#}"));
            return result;
        }
    };
    result.policy_count = sources.len();
    let input = input.to_string();
    match tokio::task::spawn_blocking(move || evaluate(&sources, &input)).await {
        Ok(Ok(findings)) => {
            result.status = set_status(&findings).into();
            result.findings = findings;
        }
        Ok(Err(e)) => {
            result.status = "errored".into();
            result.error = Some(format!("{e:#}"));
        }
        Err(e) => {
            result.status = "errored".into();
            result.error = Some(e.to_string());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: &str = r#"{"resource_changes":[
        {"address":"aws_s3_bucket.a","type":"aws_s3_bucket","change":{"actions":["create"],"after":{"tags":{}}}},
        {"address":"aws_instance.web","type":"aws_instance","change":{"actions":["delete"],"after":null}}]}"#;

    #[test]
    fn v1_deny_and_warn() {
        let src = vec![(
            "s3.rego".to_string(),
            r#"package terraform.s3
import rego.v1
deny contains msg if {
  some rc in input.resource_changes
  rc.type == "aws_s3_bucket"
  not rc.change.after.tags.owner
  msg := sprintf("%s must have an owner tag", [rc.address])
}
warn contains {"msg": "deletes a resource", "resource": rc.address} if {
  some rc in input.resource_changes
  "delete" in rc.change.actions
}"#
            .to_string(),
        )];
        let f = evaluate(&src, PLAN).unwrap();
        assert_eq!(f.len(), 2);
        assert!(f.iter().any(|x| x.level == "fail" && x.message == "aws_s3_bucket.a must have an owner tag"));
        let w = f.iter().find(|x| x.level == "warn").unwrap();
        assert_eq!(w.message, "deletes a resource");
        assert_eq!(w.details.as_ref().unwrap()["resource"], "aws_instance.web");
        assert_eq!(set_status(&f), "failed");
    }

    #[test]
    fn v0_conftest_style_is_accepted() {
        let src = vec![(
            "main.rego".to_string(),
            "package main\n\ndeny[msg] {\n  rc := input.resource_changes[_]\n  rc.change.actions[_] == \"delete\"\n  msg := sprintf(\"%s would be deleted\", [rc.address])\n}\n"
                .to_string(),
        )];
        let f = evaluate(&src, PLAN).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].message, "aws_instance.web would be deleted");
        assert_eq!(f[0].package, "main");
    }

    #[test]
    fn passing_and_syntax_error() {
        let ok = vec![("ok.rego".to_string(), "package p\nimport rego.v1\ndeny contains m if { m := \"x\"; false }".to_string())];
        assert_eq!(set_status(&evaluate(&ok, PLAN).unwrap()), "passed");
        let bad = vec![("bad.rego".to_string(), "package p\ndeny contains m if {".to_string())];
        assert!(compile(&bad).is_err());
    }
}
