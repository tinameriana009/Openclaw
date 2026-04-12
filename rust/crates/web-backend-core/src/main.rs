use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use web_backend_core::{app, StorePaths, WebBackendStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = env::var("CLAW_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let bind_address = env::var("CLAW_WEBD_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let socket_addr: SocketAddr = bind_address.parse()?;
    let store = WebBackendStore::new(
        StorePaths::from_workspace_root(workspace_root),
        &bind_address,
    );
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
