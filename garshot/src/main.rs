//! garshot - Screenshot utility for the gar desktop suite.
//!
//! This is the main entry point for the garshot daemon. It can also be used
//! for one-shot captures without running the daemon.

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use garshot::capture::capture_full_screen;
use garshot::encode::encode_png;
use garshot::x11::{Connection, ShmCapture};

#[derive(Parser)]
#[command(name = "garshot", about = "Screenshot utility for the gar desktop suite")]
#[command(version, author)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run as background daemon.
    Daemon,

    /// Capture full screen (one-shot, no daemon required).
    Screen {
        /// Output file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (png, jpeg, webp).
        #[arg(short, long, default_value = "png")]
        format: String,
    },

    // TODO: Add Region and Window commands in Sprint 2-3
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    match args.command {
        None => {
            // Default: one-shot screen capture with default settings
            let format = "png";
            let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            let output = PathBuf::from(format!("screenshot-{}.{}", timestamp, format));

            one_shot_screen(&output, format)?;
            println!("{}", output.display());
        }

        Some(Command::Screen { output, format }) => {
            // One-shot screen capture
            let output = output.unwrap_or_else(|| {
                let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                PathBuf::from(format!("screenshot-{}.{}", timestamp, format))
            });

            one_shot_screen(&output, &format)?;
            println!("{}", output.display());
        }

        Some(Command::Daemon) => {
            tracing::info!("Starting garshot daemon...");
            // TODO: Implement daemon mode in Sprint 4
            tracing::warn!("Daemon mode not yet implemented");
        }
    }

    Ok(())
}

/// Perform a one-shot screen capture.
fn one_shot_screen(output: &PathBuf, format: &str) -> anyhow::Result<()> {
    tracing::info!("Capturing full screen to {}", output.display());

    // Connect to X11
    let conn = Connection::new().context("Failed to connect to X11")?;

    tracing::debug!("Screen size: {}x{}", conn.width, conn.height);

    // Create shared memory buffer
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    // Capture screen
    let result = capture_full_screen(&conn, &shm).context("Failed to capture screen")?;

    tracing::info!(
        "Captured {}x{} ({} bytes)",
        result.width,
        result.height,
        result.data.len()
    );

    // Encode and save
    match format {
        "png" => encode_png(&result.data, result.width, result.height, output)
            .context("Failed to encode PNG")?,
        _ => {
            anyhow::bail!("Unsupported format: {} (only png supported in Sprint 1)", format);
        }
    }

    tracing::info!("Saved to {}", output.display());

    Ok(())
}
