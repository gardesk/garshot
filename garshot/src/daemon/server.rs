//! Unix socket server for daemon IPC.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;

use garshot_ipc::{socket_path, Request, Response};

use super::state::DaemonState;

/// Run the daemon server.
pub async fn run_server(state: Arc<Mutex<DaemonState>>) -> anyhow::Result<()> {
    let socket_path = socket_path();

    // Remove stale socket if it exists
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    tracing::info!("Daemon listening on {}", socket_path.display());

    let shutdown_flag = {
        let state = state.lock().await;
        state.shutdown_flag()
    };

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let state = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, state).await {
                                tracing::error!("Client error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                if shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
                    tracing::info!("Shutdown requested, exiting");
                    break;
                }
            }
        }
    }

    // Cleanup socket
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    Ok(())
}

async fn handle_client(
    stream: tokio::net::UnixStream,
    state: Arc<Mutex<DaemonState>>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let request: Request = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = Response::error(format!("Invalid request: {}", e));
                let json = serde_json::to_string(&response)?;
                writer.write_all(json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                line.clear();
                continue;
            }
        };

        tracing::debug!("Received request: {:?}", request);

        // Handle request (blocking for now, could be improved with spawn_blocking)
        let response = {
            let state = state.lock().await;
            state.handle_request(request)
        };

        let json = serde_json::to_string(&response)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        line.clear();
    }

    Ok(())
}
