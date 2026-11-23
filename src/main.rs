mod api;
mod assets;
mod config;
mod crypto;
mod db;
mod error;
mod exec;
mod git;
mod models;
mod policy;
mod runner;
mod security;
mod terminal;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use clap::Parser;
use tracing_subscriber::EnvFilter;

use config::{Cli, Command, Config, ServeArgs};

pub type AppState = Arc<Inner>;

pub struct Inner {
    pub config: Config,
    pub db: sqlx::SqlitePool,
    pub crypto: crypto::Crypto,
    pub runs: runner::RunHub,
    pub canceled: Mutex<HashSet<String>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("TUFFIE_LOG").unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")))
        .init();

    let cli = Cli::parse();
    let args = match cli.command {
        Some(Command::Serve(a)) => a,
        None => ServeArgs { no_open: false, ..ServeArgs::parse_default() },
    };
    serve(args).await
}

impl ServeArgs {
    /// Defaults plus environment variables, for `tuffie` with no subcommand.
    fn parse_default() -> Self {
        #[derive(Parser)]
        struct Wrap {
            #[command(flatten)]
            a: ServeArgs,
        }
        Wrap::parse_from(["tuffie"]).a
    }
}

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let config = Config::from_args(&args)?;
    std::fs::create_dir_all(&config.data_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&config.data_dir, std::fs::Permissions::from_mode(0o700))?;
    }
    std::fs::create_dir_all(config.runs_dir())?;
    std::fs::create_dir_all(config.checkouts_dir())?;

    let db = db::connect(&config.data_dir).await?;
    let crypto = crypto::Crypto::load_or_create(&config.data_dir)?;
    let state: AppState = Arc::new(Inner {
        config: config.clone(),
        db,
        crypto,
        runs: runner::RunHub::new(),
        canceled: Mutex::new(HashSet::new()),
    });
    runner::recover(&state).await?;
    tokio::spawn(runner::drift_scheduler(state.clone()));

    let app = api::router(state.clone());
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let url = format!("http://{}", listener.local_addr()?);
    tracing::info!(%url, mode = ?config.mode, data_dir = %config.data_dir.display(), "Tuffie is ready");
    if config.open_browser {
        open_browser(&url);
    }
    axum::serve(listener, app).with_graceful_shutdown(shutdown()).await?;
    Ok(())
}

fn open_browser(url: &str) {
    let cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(cmd)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
