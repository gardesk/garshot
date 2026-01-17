//! garshotctl - CLI control tool for the garshot daemon.
//!
//! This tool sends commands to the garshot daemon via IPC (Unix domain sockets).
//! For one-shot captures without the daemon, use `garshot screen` directly.

use anyhow::Context;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing_subscriber::EnvFilter;

use garshot_ipc::{OutputMode, Request, Response};

#[derive(Parser)]
#[command(name = "garshotctl", about = "Control tool for garshot daemon")]
#[command(version, author)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Capture full screen.
    Screen {
        /// Specific monitor name.
        #[arg(short, long)]
        monitor: Option<String>,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,

        /// Copy to clipboard instead of saving to file.
        #[arg(long)]
        clipboard: bool,
    },

    /// Interactive region selection.
    Region {
        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,

        /// Copy to clipboard instead of saving to file.
        #[arg(long)]
        clipboard: bool,
    },

    /// Capture specific window.
    Window {
        /// Window ID (hex or decimal). Defaults to active window.
        #[arg(short, long)]
        id: Option<String>,

        /// Include window decorations.
        #[arg(short, long)]
        decorations: bool,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,

        /// Copy to clipboard instead of saving to file.
        #[arg(long)]
        clipboard: bool,
    },

    /// Capture currently active window.
    Active {
        /// Include window decorations.
        #[arg(short, long)]
        decorations: bool,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,

        /// Copy to clipboard instead of saving to file.
        #[arg(long)]
        clipboard: bool,
    },

    /// Show daemon status.
    Status,

    /// List available monitors.
    Monitors,

    /// Shutdown the daemon.
    Shutdown,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let args = Args::parse();

    let request = match args.command {
        Command::Screen {
            monitor,
            cursor,
            clipboard,
        } => Request::Screen {
            monitor,
            cursor,
            output: if clipboard {
                OutputMode::Clipboard
            } else {
                OutputMode::File
            },
        },

        Command::Region { cursor, clipboard } => Request::Region {
            cursor,
            output: if clipboard {
                OutputMode::Clipboard
            } else {
                OutputMode::File
            },
        },

        Command::Window {
            id,
            decorations,
            cursor,
            clipboard,
        } => {
            let window_id = id.map(|s| parse_window_id(&s)).transpose()?;
            Request::Window {
                id: window_id,
                decorations,
                cursor,
                output: if clipboard {
                    OutputMode::Clipboard
                } else {
                    OutputMode::File
                },
            }
        }

        Command::Active {
            decorations,
            cursor,
            clipboard,
        } => Request::ActiveWindow {
            decorations,
            cursor,
            output: if clipboard {
                OutputMode::Clipboard
            } else {
                OutputMode::File
            },
        },

        Command::Status => Request::Status,
        Command::Monitors => Request::ListMonitors,
        Command::Shutdown => Request::Shutdown,
    };

    let response = send_request(request).await?;

    if response.success {
        if let Some(path) = response.path {
            println!("{}", path.display());
        }
        if let Some(info) = response.info {
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
    } else {
        let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
        eprintln!("Error: {}", error_msg);
        std::process::exit(1);
    }

    Ok(())
}

/// Send a request to the garshot daemon and return the response.
async fn send_request(request: Request) -> anyhow::Result<Response> {
    let socket_path = garshot_ipc::socket_path();

    if !socket_path.exists() {
        anyhow::bail!(
            "Daemon not running (socket not found: {})\n\
             Start the daemon with: garshot daemon\n\
             Or use one-shot mode: garshot screen",
            socket_path.display()
        );
    }

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .context("Failed to connect to daemon")?;

    // Send request as JSON line
    let json = serde_json::to_string(&request)?;
    stream.write_all(json.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    // Read response
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: Response = serde_json::from_str(&line).context("Failed to parse response")?;

    Ok(response)
}

/// Parse a window ID from string (supports hex 0x... or decimal).
fn parse_window_id(s: &str) -> anyhow::Result<u32> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16).context("Invalid hex window ID")
    } else {
        s.parse().context("Invalid decimal window ID")
    }
}
