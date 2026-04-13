use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use web_backend_core::{app, export_static_status_page, StorePaths, WebBackendStore};

#[derive(Debug, Parser)]
#[command(
    name = "claw-webd",
    version,
    about = "Bounded local backend core for Claw operator state"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the localhost JSON API daemon.
    Serve,
    /// Import a staged repo-analysis bundle into backend queue/runtime state.
    ImportRepoAnalysisBundle { bundle_dir: PathBuf },
    /// Fetch the backend API and write a static HTML status page.
    ExportStaticStatusPage {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        api_base_url: String,
        #[arg(long, default_value = ".claw/backend/static-status.html")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let workspace_root = env::var("CLAW_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let bind_address = env::var("CLAW_WEBD_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let store = WebBackendStore::new(
        StorePaths::from_workspace_root(workspace_root),
        &bind_address,
    );

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(store, bind_address).await?,
        Command::ImportRepoAnalysisBundle { bundle_dir } => {
            let report = store.import_repo_analysis_bundle(bundle_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::ExportStaticStatusPage {
            api_base_url,
            output,
        } => {
            let report = export_static_status_page(&api_base_url, &output).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

async fn serve(
    store: WebBackendStore,
    bind_address: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let socket_addr: SocketAddr = bind_address.parse()?;
    store.ensure_storage()?;

    let listener = tokio::net::TcpListener::bind(socket_addr).await?;
    println!(
        "claw-webd listening on http://{}\nLocal backend core only: persisted operator state + runtime snapshots, not a full live web app.",
        listener.local_addr()?
    );
    axum::serve(listener, app(store))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    ctrl_c.await;
}
