#!/usr/bin/env bash
# End-to-end smoke test. Uses tests/fixtures/mock-terraform, so no real
# Terraform or cloud account is needed.
set -euo pipefail
cd "$(dirname "$0")/.."
BIN=${BIN:-./target/debug/tuffie}
TMP=$(mktemp -d)
PORT=${PORT:-7791}
B="http://127.0.0.1:$PORT/api"
MOCK="$PWD/tests/fixtures/mock-terraform"
trap 'kill $PID 2>/dev/null || true; rm -rf "$TMP"' EXIT

mkdir -p "$TMP/ws" && cp examples/basic/main.tf "$TMP/ws/"
TUFFIE_LOG=warn "$BIN" serve --data-dir "$TMP/data" --listen "127.0.0.1:$PORT" --no-open &
PID=$!
for _ in $(seq 50); do curl -sf "$B/system/info" >/dev/null && break; sleep 0.1; done

json() { python3 -c "import json,sys; print(json.load(sys.stdin)$1)"; }
fail() { echo "FAIL: $*"; exit 1; }

WS=$(curl -sf -XPOST "$B/workspaces" -H 'content-type: application/json' \
  -d "{\"name\":\"smoke\",\"source_kind\":\"local\",\"local_path\":\"$TMP/ws\",\"tool_path\":\"$MOCK\"}" | json '["id"]')
curl -sf -XPOST "$B/workspaces/$WS/variables" -H 'content-type: application/json' \
  -d '{"key":"pw","value":"hunter2","category":"terraform","sensitive":true}' >/dev/null

RUN=$(curl -sf -XPOST "$B/workspaces/$WS/runs" -H 'content-type: application/json' -d '{"kind":"plan_and_apply"}' | json '["id"]')
for _ in $(seq 50); do [ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = planned ] && break; sleep 0.1; done
[ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = planned ] || fail "run did not reach planned"
curl -sf "$B/runs/$RUN/plan" | grep -q '"(sensitive value)"' || fail "sensitive plan value not masked"

curl -sf -XPOST "$B/runs/$RUN/apply" -o /dev/null
for _ in $(seq 50); do [ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = applied ] && break; sleep 0.1; done
[ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = applied ] || fail "run did not apply"
[ "$(curl -sf "$B/runs/$RUN" | json '["has_state_snapshot"]')" = True ] || fail "no state snapshot"

grep -rq hunter2 "$TMP/data/runs" && fail "plaintext secret found in run files"
[ "$(curl -s -o /dev/null -w '%{http_code}' -H 'Host: evil.example' "$B/system/info")" = 403 ] || fail "bad Host accepted"
[ "$(curl -s -o /dev/null -w '%{http_code}' -XPOST -H 'Origin: http://evil.example' "$B/workspaces" -d '{}')" = 403 ] || fail "cross-origin POST accepted"
[ "$(curl -s -o /dev/null -w '%{http_code}' "$B/workspaces/$WS/files/content?path=../../etc/passwd")" = 400 ] || fail "path traversal accepted"
[ "$(curl -s -o /dev/null -w '%{http_code}' "$B/workspaces/$WS/files/content?path=terraform.tfstate")" = 403 ] || fail "state file opened in editor"

# --- Variable sets: a global set; the workspace value wins on conflict.
VS=$(curl -sf -XPOST "$B/variable-sets" -H 'content-type: application/json' -d '{"name":"defaults","global":true}' | json '["id"]')
curl -sf -XPOST "$B/variable-sets/$VS/variables" -H 'content-type: application/json' -d '{"key":"pw","value":"from-set","category":"terraform"}' >/dev/null
curl -sf -XPOST "$B/variable-sets/$VS/variables" -H 'content-type: application/json' -d '{"key":"region","value":"eu-west-1","category":"terraform"}' >/dev/null

# --- Mandatory policy that fails on the mock plan (an S3 bucket without an owner tag).
PS=$(curl -sf -XPOST "$B/policy-sets" -H 'content-type: application/json' -d "{\"name\":\"guard\",\"enforcement\":\"mandatory\",\"workspace_ids\":[\"$WS\"]}" | json '["id"]')
python3 - "$B/policy-sets/$PS/policies" <<'PY'
import json, sys, urllib.request
rego = 'package t\nimport rego.v1\ndeny contains m if {\n  some rc in input.resource_changes\n  rc.type == "aws_s3_bucket"\n  not rc.change.after.tags.owner\n  m := "needs owner"\n}\n'
req = urllib.request.Request(sys.argv[1], data=json.dumps({"name": "owner", "rego": rego}).encode(), headers={"content-type": "application/json"}, method="POST")
urllib.request.urlopen(req)
PY
rm -f "$TMP/ws/terraform.tfstate"
RUN2=$(curl -sf -XPOST "$B/workspaces/$WS/runs" -H 'content-type: application/json' -d '{"kind":"plan_and_apply"}' | json '["id"]')
for _ in $(seq 50); do [ "$(curl -sf "$B/runs/$RUN2" | json '["status"]')" = policy_failed ] && break; sleep 0.1; done
[ "$(curl -sf "$B/runs/$RUN2" | json '["status"]')" = policy_failed ] || fail "mandatory policy did not block the apply"
[ "$(curl -sf "$B/workspaces/$WS" | json '["lock_run_id"]')" = None ] || fail "lock not released after policy failure"
curl -sf "$B/runs/$RUN2/variables" | python3 -c '
import json, sys
o = {(x["key"], x["source"]): x for x in json.load(sys.stdin)}
assert o[("pw", "variable_set")]["overridden"], "workspace should override the set"
assert not o[("region", "variable_set")]["overridden"]
' || fail "variable origins are wrong"

# --- Drift: MOCK_DRIFT makes the mock report drift on a refresh-only plan.
curl -sf -XDELETE "$B/policy-sets/$PS" -o /dev/null
curl -sf -XPOST "$B/workspaces/$WS/variables" -H 'content-type: application/json' -d '{"key":"MOCK_DRIFT","value":"1","category":"env"}' >/dev/null
curl -sf -XPOST "$B/workspaces/$WS/drift" -o /dev/null
for _ in $(seq 50); do [ "$(curl -sf "$B/workspaces/$WS" | json '["drift_status"]')" = drifted ] && break; sleep 0.1; done
[ "$(curl -sf "$B/workspaces/$WS" | json '["drift_status"]')" = drifted ] || fail "drift not detected"

echo "smoke test passed"
