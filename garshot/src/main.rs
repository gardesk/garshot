//! garshot - Screenshot utility for the gar desktop suite.
//!
//! This is the main entry point for the garshot daemon. It can also be used
//! for one-shot captures without running the daemon.

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use garshot::annotate::{AnnotationOverlay, AnnotationResult};
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
        /// Output file path (use "-" for stdout).
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

        /// Don't copy to clipboard (default: copy to clipboard).
        #[arg(long)]
        no_clipboard: bool,

        /// Delay in seconds before capture.
        #[arg(long)]
        delay: Option<u64>,

        /// Show desktop notification on save.
        #[arg(long)]
        notify: bool,

        /// Open annotation editor after capture.
        #[arg(short, long)]
        annotate: bool,
    },

    /// Capture a region by geometry.
    Region {
        /// Region geometry: WxH+X+Y (e.g., 800x600+100+50).
        #[arg(short, long)]
        geometry: String,

        /// Output file path (use "-" for stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format.
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,

        /// Don't copy to clipboard (default: copy to clipboard).
        #[arg(long)]
        no_clipboard: bool,

        /// Delay in seconds before capture.
        #[arg(long)]
        delay: Option<u64>,

        /// Show desktop notification on save.
        #[arg(long)]
        notify: bool,

        /// Open annotation editor after capture.
        #[arg(short, long)]
        annotate: bool,
    },

    /// Capture a window.
    Window {
        /// Window ID (hex or decimal). Defaults to active window.
        #[arg(short, long)]
        id: Option<String>,

        /// Include window decorations.
        #[arg(short, long)]
        decorations: bool,

        /// Output file path (use "-" for stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format.
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Include cursor in screenshot.
        #[arg(short, long)]
        cursor: bool,

        /// Don't copy to clipboard (default: copy to clipboard).
        #[arg(long)]
        no_clipboard: bool,

        /// Delay in seconds before capture.
        #[arg(long)]
        delay: Option<u64>,

        /// Show desktop notification on save.
        #[arg(long)]
        notify: bool,

        /// Open annotation editor after capture.
        #[arg(short, long)]
        annotate: bool,
    },

    /// Interactive region selection with blur overlay.
    Select {
        /// Output file path (use "-" for stdout).
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

        /// Don't copy to clipboard (default: copy to clipboard).
        #[arg(long)]
        no_clipboard: bool,

        /// Show desktop notification on save.
        #[arg(long)]
        notify: bool,

        /// Open annotation editor after capture.
        #[arg(short, long)]
        annotate: bool,
    },

    /// Annotate an existing image file.
    Annotate {
        /// Input image file to annotate.
        file: PathBuf,

        /// Output file path (default: overwrites input, use "-" for stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (defaults to input format).
        #[arg(short, long)]
        format: Option<String>,
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
            copy_to_clipboard(&output)?;
            println!("{}", output.display());
        }

        Some(Command::Screen {
            output,
            format,
            monitor,
            cursor,
            no_clipboard,
            delay,
            notify,
            annotate,
        }) => {
            if let Some(secs) = delay {
                sleep_with_countdown(secs);
            }
            let is_stdout = output.as_ref().map(|p| p.as_os_str() == "-").unwrap_or(false);
            let output_path = if is_stdout {
                None
            } else {
                Some(output.unwrap_or_else(|| default_output(&config, &format)))
            };
            let (mut data, mut width, mut height) = capture_screen_raw(&format, monitor.as_deref(), cursor)?;

            // Open annotation editor if requested
            if annotate {
                match run_annotation(&data, width, height)? {
                    Some((new_data, new_w, new_h)) => {
                        data = new_data;
                        width = new_w;
                        height = new_h;
                    }
                    None => {
                        tracing::info!("Annotation cancelled");
                        return Ok(());
                    }
                }
            }

            if let Some(ref path) = output_path {
                save_image(&data, width, height, path, &format)?;
                if !no_clipboard {
                    copy_to_clipboard(path)?;
                }
                if notify {
                    send_notification(path, width, height);
                }
                println!("{}", path.display());
            } else {
                write_stdout(&data, width, height, &format)?;
            }
        }

        Some(Command::Region {
            geometry,
            output,
            format,
            cursor,
            no_clipboard,
            delay,
            notify,
            annotate,
        }) => {
            if let Some(secs) = delay {
                sleep_with_countdown(secs);
            }
            let is_stdout = output.as_ref().map(|p| p.as_os_str() == "-").unwrap_or(false);
            let output_path = if is_stdout {
                None
            } else {
                Some(output.unwrap_or_else(|| default_output(&config, &format)))
            };
            let (mut data, mut width, mut height) = capture_region_raw(&geometry, cursor)?;

            // Open annotation editor if requested
            if annotate {
                match run_annotation(&data, width, height)? {
                    Some((new_data, new_w, new_h)) => {
                        data = new_data;
                        width = new_w;
                        height = new_h;
                    }
                    None => {
                        tracing::info!("Annotation cancelled");
                        return Ok(());
                    }
                }
            }

            if let Some(ref path) = output_path {
                save_image(&data, width, height, path, &format)?;
                if !no_clipboard {
                    copy_to_clipboard(path)?;
                }
                if notify {
                    send_notification(path, width, height);
                }
                println!("{}", path.display());
            } else {
                write_stdout(&data, width, height, &format)?;
            }
        }

        Some(Command::Window {
            id,
            decorations,
            output,
            format,
            cursor,
            no_clipboard,
            delay,
            notify,
            annotate,
        }) => {
            if let Some(secs) = delay {
                sleep_with_countdown(secs);
            }
            let is_stdout = output.as_ref().map(|p| p.as_os_str() == "-").unwrap_or(false);
            let output_path = if is_stdout {
                None
            } else {
                Some(output.unwrap_or_else(|| default_output(&config, &format)))
            };
            let (mut data, mut width, mut height) = capture_window_raw(id.as_deref(), decorations, cursor)?;

            // Open annotation editor if requested
            if annotate {
                match run_annotation(&data, width, height)? {
                    Some((new_data, new_w, new_h)) => {
                        data = new_data;
                        width = new_w;
                        height = new_h;
                    }
                    None => {
                        tracing::info!("Annotation cancelled");
                        return Ok(());
                    }
                }
            }

            if let Some(ref path) = output_path {
                save_image(&data, width, height, path, &format)?;
                if !no_clipboard {
                    copy_to_clipboard(path)?;
                }
                if notify {
                    send_notification(path, width, height);
                }
                println!("{}", path.display());
            } else {
                write_stdout(&data, width, height, &format)?;
            }
        }

        Some(Command::Select {
            output,
            format,
            cursor,
            blur,
            no_clipboard,
            notify,
            annotate,
        }) => {
            let is_stdout = output.as_ref().map(|p| p.as_os_str() == "-").unwrap_or(false);
            let output_path = if is_stdout {
                None
            } else {
                Some(output.unwrap_or_else(|| default_output(&config, &format)))
            };
            if let Some((mut data, mut width, mut height)) = capture_select_raw(cursor, blur)? {
                // Open annotation editor if requested
                if annotate {
                    match run_annotation(&data, width, height)? {
                        Some((new_data, new_w, new_h)) => {
                            data = new_data;
                            width = new_w;
                            height = new_h;
                        }
                        None => {
                            tracing::info!("Annotation cancelled");
                            return Ok(());
                        }
                    }
                }

                if let Some(ref path) = output_path {
                    save_image(&data, width, height, path, &format)?;
                    if !no_clipboard {
                        copy_to_clipboard(path)?;
                    }
                    if notify {
                        send_notification(path, width, height);
                    }
                    println!("{}", path.display());
                } else {
                    write_stdout(&data, width, height, &format)?;
                }
            }
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

        Some(Command::Annotate { file, output, format }) => {
            // Load image file
            let img = image::open(&file).context("Failed to open image file")?;
            let rgba = img.to_rgba8();
            let width = rgba.width();
            let height = rgba.height();
            let data = rgba.into_raw();

            tracing::info!("Opening annotation editor for {}", file.display());

            // Run annotation
            match run_annotation(&data, width, height)? {
                Some((annotated_data, w, h)) => {
                    // Determine output path and format
                    let out_path = output.unwrap_or_else(|| file.clone());
                    let is_stdout = out_path.as_os_str() == "-";
                    let out_format = format.unwrap_or_else(|| {
                        file.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("png")
                            .to_string()
                    });

                    if is_stdout {
                        write_stdout(&annotated_data, w, h, &out_format)?;
                    } else {
                        save_image(&annotated_data, w, h, &out_path, &out_format)?;
                        println!("{}", out_path.display());
                    }
                }
                None => {
                    tracing::info!("Annotation cancelled");
                }
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

fn copy_to_clipboard(path: &PathBuf) -> anyhow::Result<()> {
    let mime_type = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ppm") | Some("pam") => "image/x-portable-pixmap",
        _ => "image/png", // Default to PNG
    };

    // Try xclip first, then xsel
    let result = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-t", mime_type, "-i"])
        .arg(path)
        .status();

    match result {
        Ok(status) if status.success() => {
            tracing::debug!("Copied {} to clipboard via xclip", path.display());
            Ok(())
        }
        _ => {
            // Fallback to xsel (doesn't support MIME types as well)
            let result = std::process::Command::new("xsel")
                .args(["--clipboard", "--input"])
                .stdin(std::fs::File::open(path)?)
                .status();

            match result {
                Ok(status) if status.success() => {
                    tracing::debug!("Copied {} to clipboard via xsel", path.display());
                    Ok(())
                }
                Ok(status) => {
                    tracing::warn!("xsel exited with status: {}", status);
                    anyhow::bail!("Failed to copy to clipboard")
                }
                Err(e) => {
                    tracing::warn!("Neither xclip nor xsel available: {}", e);
                    anyhow::bail!("No clipboard tool available (install xclip or xsel)")
                }
            }
        }
    }
}

fn sleep_with_countdown(secs: u64) {
    for i in (1..=secs).rev() {
        eprint!("\rCapturing in {}... ", i);
        std::io::stderr().flush().ok();
        std::thread::sleep(Duration::from_secs(1));
    }
    eprintln!("\rCapturing now!     ");
}

fn send_notification(path: &PathBuf, width: u32, height: u32) {
    let summary = "Screenshot saved";
    let body = format!("{}x{} → {}", width, height, path.display());

    // Try notify-send (most common)
    let result = std::process::Command::new("notify-send")
        .args(["-i", "camera-photo", "-a", "garshot", summary, &body])
        .status();

    if result.is_err() || !result.unwrap().success() {
        tracing::debug!("notify-send not available or failed");
    }
}

fn write_stdout(data: &[u8], width: u32, height: u32, format: &str) -> anyhow::Result<()> {
    let encoded = encode_to_vec(data, width, height, format)?;
    std::io::stdout().write_all(&encoded)?;
    std::io::stdout().flush()?;
    Ok(())
}

fn encode_to_vec(data: &[u8], width: u32, height: u32, format: &str) -> anyhow::Result<Vec<u8>> {
    garshot::encode::encode_to_vec(data, width, height, format, 90)
        .context("Failed to encode image")
}

fn capture_screen_raw(
    _format: &str,
    monitor: Option<&str>,
    include_cursor: bool,
) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    let conn = Connection::new().context("Failed to connect to X11")?;
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    let mut result = if let Some(monitor_name) = monitor {
        tracing::info!("Capturing monitor {}", monitor_name);
        capture_monitor(&conn, &shm, monitor_name)?
    } else {
        tracing::info!("Capturing full screen");
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

    Ok((result.data, result.width, result.height))
}

fn capture_region_raw(
    geometry: &str,
    include_cursor: bool,
) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    let conn = Connection::new().context("Failed to connect to X11")?;
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    let region = Region::from_geometry(geometry).context("Invalid geometry")?;
    tracing::info!(
        "Capturing region {}x{}+{}+{}",
        region.width,
        region.height,
        region.x,
        region.y
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

    Ok((result.data, result.width, result.height))
}

fn capture_window_raw(
    window_id: Option<&str>,
    decorations: bool,
    include_cursor: bool,
) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    use garshot::capture::{capture_active_window, capture_window, get_active_window};

    let conn = Connection::new().context("Failed to connect to X11")?;
    let buffer_size = conn.width as usize * conn.height as usize * 4;
    let shm = ShmCapture::new(&conn, buffer_size).context("Failed to create SHM buffer")?;

    let mut result = if let Some(id_str) = window_id {
        let id = parse_window_id(id_str)?;
        tracing::info!("Capturing window 0x{:x}", id);
        capture_window(&conn, &shm, id, decorations)?
    } else {
        let active = get_active_window(&conn)?;
        tracing::info!("Capturing active window 0x{:x}", active);
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

    Ok((result.data, result.width, result.height))
}

fn capture_select_raw(
    include_cursor: bool,
    blur_radius: usize,
) -> anyhow::Result<Option<(Vec<u8>, u32, u32)>> {
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
            return Ok(None);
        }
    };

    tracing::info!(
        "Selected region {}x{}+{}+{}",
        region.width,
        region.height,
        region.x,
        region.y
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

    Ok(Some((result.data, result.width, result.height)))
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

/// Run the annotation overlay and return the annotated image data.
/// Returns None if the user cancelled.
fn run_annotation(
    data: &[u8],
    width: u32,
    height: u32,
) -> anyhow::Result<Option<(Vec<u8>, u32, u32)>> {
    let overlay = AnnotationOverlay::new(data, width, height)
        .context("Failed to create annotation overlay")?;

    match overlay.run().context("Annotation overlay failed")? {
        AnnotationResult::Save { data, width, height } => {
            Ok(Some((data, width, height)))
        }
        AnnotationResult::Cancel => Ok(None),
    }
}
