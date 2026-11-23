use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "tuffie", version, about = "A fast, local-first UI for Terraform and OpenTofu")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start the Tuffie server and web UI.
    Serve(ServeArgs),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Single user on one machine. No login. Listens on loopback only.
    Local,
    /// Shared server for a team. Login is required.
    Server,
}

#[derive(clap::Args, Debug, Clone)]
pub struct ServeArgs {
    /// Address to listen on.
    #[arg(long, env = "TUFFIE_LISTEN", default_value = "127.0.0.1:7777")]
    pub listen: SocketAddr,

    /// Folder for the database, run logs, plan files and the master key.
    #[arg(long, env = "TUFFIE_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// Run mode.
    #[arg(long, env = "TUFFIE_MODE", value_enum, default_value_t = Mode::Local)]
    pub mode: Mode,

    /// Extra host names that the server accepts in the Host header
    /// (for example `tuffie.internal`). Loopback names are always accepted.
    #[arg(long = "allowed-host", env = "TUFFIE_ALLOWED_HOSTS", value_delimiter = ',')]
    pub allowed_hosts: Vec<String>,

    /// Let Terraform processes inherit the environment of the Tuffie process.
    /// On by default in local mode so that `~/.aws`, `AWS_PROFILE` and similar work
    /// exactly like in your shell. Off by default in server mode.
    #[arg(long, env = "TUFFIE_INHERIT_ENV")]
    pub inherit_env: Option<bool>,

    /// Do not open the browser on start.
    #[arg(long, env = "TUFFIE_NO_OPEN")]
    pub no_open: bool,

    /// Allow server mode without login. Only for testing behind your own auth proxy.
    #[arg(long, env = "TUFFIE_UNSAFE_NO_AUTH")]
    pub unsafe_no_auth: bool,
}

impl Default for ServeArgs {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:7777".parse().unwrap(),
            data_dir: None,
            mode: Mode::Local,
            allowed_hosts: vec![],
            inherit_env: None,
            no_open: true,
            unsafe_no_auth: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    pub mode: Mode,
    pub allowed_hosts: Vec<String>,
    pub inherit_env: bool,
    pub open_browser: bool,
}

impl Config {
    pub fn from_args(args: &ServeArgs) -> anyhow::Result<Self> {
        let data_dir = match &args.data_dir {
            Some(d) => d.clone(),
            None => dirs::data_local_dir()
                .or_else(dirs::home_dir)
                .ok_or_else(|| anyhow::anyhow!("cannot find a data folder; pass --data-dir"))?
                .join("tuffie"),
        };

        if args.mode == Mode::Server && !args.unsafe_no_auth {
            anyhow::bail!(
                "server mode needs SSO login, which is not implemented yet (see the roadmap). \
                 Use local mode, or pass --unsafe-no-auth only behind your own authenticating proxy."
            );
        }
        if args.mode == Mode::Local && !args.listen.ip().is_loopback() {
            anyhow::bail!(
                "local mode has no login, so it only listens on a loopback address \
                 (for example 127.0.0.1). Use --mode server to listen on {}.",
                args.listen
            );
        }

        Ok(Self {
            listen: args.listen,
            data_dir,
            mode: args.mode,
            allowed_hosts: args.allowed_hosts.iter().map(|h| h.to_ascii_lowercase()).collect(),
            inherit_env: args.inherit_env.unwrap_or(args.mode == Mode::Local),
            open_browser: !args.no_open,
        })
    }

    pub fn runs_dir(&self) -> PathBuf {
        self.data_dir.join("runs")
    }

    pub fn checkouts_dir(&self) -> PathBuf {
        self.data_dir.join("checkouts")
    }
}
