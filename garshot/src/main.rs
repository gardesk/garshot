//! garshot - Screenshot utility for the gar desktop suite.
//!
//! This is the main entry point for the garshot daemon. It can also be used
//! for one-shot captures without running the daemon.

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use garshot::capture::{
    blend_cursor, capture_full_screen, capture_region, get_cursor_image, Region,
};
use garshot::config::{load_config, Config};
use garshot::encode::encode_png;
use garshot::selection::overlay::{interactive_selection, SelectionConfig};
use garshot::x11::{capture_monitor, get_monitors, Connection, ShmCapture};

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

        /// Specific monitor name.
        #[arg(short, long)]
        monitor: Option<String>,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,
    },

    /// Capture a region by geometry.
    Region {
        /// Region geometry: WxH+X+Y (e.g., 800x600+100+50).
        #[arg(short, long)]
        geometry: String,

        /// Output file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format.
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,
    },

    /// Capture a window.
    Window {
        /// Window ID (hex or decimal). Defaults to active window.
        #[arg(short, long)]
        id: Option<String>,

        /// Include window decorations.
        #[arg(short, long)]
        decorations: bool,

        /// Output file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format.
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,
    },

    /// Interactive region selection with blur overlay.
    Select {
        /// Output file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format.
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,

        /// Blur radius for overlay (default: 15).
        #[arg(short, long, default_value = "15")]
        blur: usize,
    },

    /// List available monitors.
    Monitors,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Load config for default values
    let config = load_config().unwrap_or_else(|e| {
        tracing::debug!("Failed to load config: {}, using defaults", e);
        Config::default()
    });

    match args.command {
        None => {
            let format = &config.general.format;
            let output = default_output(&config, format);
            capture_screen(&output, format, None, config.general.include_cursor)?;
            println!("{}", output.display());
        }

        Some(Command::Screen {
            output,
            format,
            monitor,
            cursor,
        }) => {
            let output = output.unwrap_or_else(|| default_output(&config, &format));
            capture_screen(&output, &format, monitor.as_deref(), cursor)?;
            println!("{}", output.display());
        }

        Some(Command::Region {
            geometry,
            output,
            format,
            cursor,
        }) => {
            let output = output.unwrap_or_else(|| default_output(&config, &format));
            capture_region_cmd(&output, &format, &geometry, cursor)?;
            println!("{}", output.display());
        }

        Some(Command::Window {
            id,
            decorations,
            output,
            format,
            cursor,
        }) => {
            let output = output.unwrap_or_else(|| default_output(&config, &format));
            capture_window_cmd(&output, &format, id.as_deref(), decorations, cursor)?;
            println!("{}", output.display());
        }

        Some(Command::Select {
            output,
            format,
            cursor,
            blur,
        }) => {
            let output = output.unwrap_or_else(|| default_output(&config, &format));
            capture_select_cmd(&output, &format, cursor, blur)?;
            println!("{}", output.display());
        }

        Some(Command::Monitors) => {
            let conn = Connection::new().context("Failed to connect to X11")?;
            let monitors = get_monitors(&conn)?;
            for m in monitors {
                println!(
                    "{} {}x{}+{}+{}{}",
                    m.name,
                    m.width,
                    m.height,
                    m.x,
                    m.y,
                    if m.primary { " (primary)" } else { "" }
                );
            }
        }

        Some(Command::Daemon) => {
            run_daemon()?;
        }
    }

    Ok(())
}

fn default_output(config: &Config, format: &str) -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!("screenshot-{}.{}", timestamp, format);

    // Ensure save directory exists
    let save_dir = &config.general.save_dir;
    if let Err(e) = std::fs::create_dir_all(save_dir) {
        tracing::warn!("Failed to create save directory {}: {}", save_dir.display(), e);
        return PathBuf::from(filename);
    }

    save_dir.join(filename)
}

fn capture_screen(
    output: &PathBuf,
    format: &str,
    monitor: Option<&str>,
    include_cursor: bool,
) -> anyhow::Result<()> {
    let conn = Connection::new().context("Failed to connect to X11")?;
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    let mut result = if let Some(monitor_name) = monitor {
        tracing::info!("Capturing monitor {} to {}", monitor_name, output.display());
        capture_monitor(&conn, &shm, monitor_name)?
    } else {
        tracing::info!("Capturing full screen to {}", output.display());
        let r = capture_full_screen(&conn, &shm)?;
        garshot::capture::RegionCaptureResult {
            data: r.data,
            width: r.width,
            height: r.height,
            region: Region::new(0, 0, conn.width, conn.height),
        }
    };

    if include_cursor {
        if let Ok(cursor) = get_cursor_image(&conn) {
            blend_cursor(
                &mut result.data,
                result.width,
                result.height,
                &result.region,
                &cursor,
            );
        }
    }

    save_image(&result.data, result.width, result.height, output, format)
}

fn capture_region_cmd(
    output: &PathBuf,
    format: &str,
    geometry: &str,
    include_cursor: bool,
) -> anyhow::Result<()> {
    let conn = Connection::new().context("Failed to connect to X11")?;
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    let region = Region::from_geometry(geometry).context("Invalid geometry")?;
    tracing::info!(
        "Capturing region {}x{}+{}+{} to {}",
        region.width,
        region.height,
        region.x,
        region.y,
        output.display()
    );

    let mut result = capture_region(&conn, &shm, &region)?;

    if include_cursor {
        if let Ok(cursor) = get_cursor_image(&conn) {
            blend_cursor(
                &mut result.data,
                result.width,
                result.height,
                &result.region,
                &cursor,
            );
        }
    }

    save_image(&result.data, result.width, result.height, output, format)
}

fn capture_window_cmd(
    output: &PathBuf,
    format: &str,
    window_id: Option<&str>,
    decorations: bool,
    include_cursor: bool,
) -> anyhow::Result<()> {
    use garshot::capture::{capture_active_window, capture_window, get_active_window};

    let conn = Connection::new().context("Failed to connect to X11")?;
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    let mut result = if let Some(id_str) = window_id {
        let id = parse_window_id(id_str)?;
        tracing::info!("Capturing window 0x{:x} to {}", id, output.display());
        capture_window(&conn, &shm, id, decorations)?
    } else {
        let active = get_active_window(&conn)?;
        tracing::info!("Capturing active window 0x{:x} to {}", active, output.display());
        capture_active_window(&conn, &shm, decorations)?
    };

    if include_cursor {
        if let Ok(cursor) = get_cursor_image(&conn) {
            blend_cursor(
                &mut result.data,
                result.width,
                result.height,
                &result.region,
                &cursor,
            );
        }
    }

    save_image(&result.data, result.width, result.height, output, format)
}

fn save_image(
    data: &[u8],
    width: u32,
    height: u32,
    output: &PathBuf,
    format: &str,
) -> anyhow::Result<()> {
    match format {
        "png" => encode_png(data, width, height, output).context("Failed to encode PNG")?,
        _ => anyhow::bail!("Unsupported format: {}", format),
    }
    tracing::info!("Saved {}x{} to {}", width, height, output.display());
    Ok(())
}

fn parse_window_id(s: &str) -> anyhow::Result<u32> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16).context("Invalid hex window ID")
    } else {
        s.parse().context("Invalid decimal window ID")
    }
}

fn capture_select_cmd(
    output: &PathBuf,
    format: &str,
    include_cursor: bool,
    blur_radius: usize,
) -> anyhow::Result<()> {
    let conn = Connection::new().context("Failed to connect to X11")?;
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    let config = SelectionConfig {
        blur_radius,
        ..Default::default()
    };

    tracing::info!("Starting interactive selection");

    let region = match interactive_selection(&conn, &shm, &config)? {
        Some(region) => region,
        None => {
            tracing::info!("Selection cancelled");
            return Ok(());
        }
    };

    tracing::info!(
        "Selected region {}x{}+{}+{}",
        region.width,
        region.height,
        region.x,
        region.y
    );

    // Capture the selected region
    let mut result = capture_region(&conn, &shm, &region)?;

    if include_cursor {
        if let Ok(cursor) = get_cursor_image(&conn) {
            blend_cursor(
                &mut result.data,
                result.width,
                result.height,
                &result.region,
                &cursor,
            );
        }
    }

    save_image(&result.data, result.width, result.height, output, format)
}

fn run_daemon() -> anyhow::Result<()> {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use garshot::daemon::{run_server, DaemonState};

    tracing::info!("Starting garshot daemon...");

    let state = DaemonState::new().context("Failed to initialize daemon state")?;
    let state = Arc::new(Mutex::new(state));

    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;
    rt.block_on(async {
        run_server(state).await
    })?;

    tracing::info!("Daemon stopped");
    Ok(())
}
