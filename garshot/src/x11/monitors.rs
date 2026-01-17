//! Multi-monitor support via XRandR.

use x11rb::protocol::randr::{self, ConnectionExt as RandrExt};

use crate::capture::region::{capture_region, Region, RegionCaptureResult};
use crate::error::{GarshotError, Result};
use crate::x11::{Connection, ShmCapture};

/// Information about a monitor.
#[derive(Debug, Clone)]
pub struct Monitor {
    /// Output name (e.g., "HDMI-1", "eDP-1").
    pub name: String,
    /// X coordinate.
    pub x: i16,
    /// Y coordinate.
    pub y: i16,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Whether this is the primary monitor.
    pub primary: bool,
    /// Output ID.
    pub output_id: randr::Output,
    /// CRTC ID.
    pub crtc_id: randr::Crtc,
}

impl Monitor {
    /// Convert to a capture region.
    pub fn to_region(&self) -> Region {
        Region::new(self.x, self.y, self.width, self.height)
    }
}

/// Query XRandR for available monitors.
pub fn get_monitors(conn: &Connection) -> Result<Vec<Monitor>> {
    // Query RandR version
    let version = conn
        .conn
        .randr_query_version(1, 5)?
        .reply()
        .map_err(|_| GarshotError::RandrNotAvailable)?;

    tracing::debug!(
        "XRandR version {}.{}",
        version.major_version,
        version.minor_version
    );

    // Get screen resources
    let resources = conn
        .conn
        .randr_get_screen_resources(conn.root)?
        .reply()?;

    // Get primary output
    let primary_output = conn.conn.randr_get_output_primary(conn.root)?.reply()?.output;

    let mut monitors = Vec::new();

    for &output in &resources.outputs {
        let output_info = match conn.conn.randr_get_output_info(output, 0)?.reply() {
            Ok(info) => info,
            Err(_) => continue,
        };

        // Skip disconnected or disabled outputs
        if output_info.crtc == 0 || output_info.connection != randr::Connection::CONNECTED {
            continue;
        }

        let crtc_info = match conn.conn.randr_get_crtc_info(output_info.crtc, 0)?.reply() {
            Ok(info) => info,
            Err(_) => continue,
        };

        let name = String::from_utf8_lossy(&output_info.name).to_string();

        monitors.push(Monitor {
            name,
            x: crtc_info.x,
            y: crtc_info.y,
            width: crtc_info.width,
            height: crtc_info.height,
            primary: output == primary_output,
            output_id: output,
            crtc_id: output_info.crtc,
        });
    }

    // Sort by position (left to right, top to bottom)
    monitors.sort_by(|a, b| {
        if a.x != b.x {
            a.x.cmp(&b.x)
        } else {
            a.y.cmp(&b.y)
        }
    });

    tracing::debug!("Found {} monitors", monitors.len());
    for m in &monitors {
        tracing::debug!(
            "  {} {}x{}+{}+{} {}",
            m.name,
            m.width,
            m.height,
            m.x,
            m.y,
            if m.primary { "(primary)" } else { "" }
        );
    }

    Ok(monitors)
}

/// Find a monitor by name.
pub fn find_monitor<'a>(monitors: &'a [Monitor], name: &str) -> Option<&'a Monitor> {
    monitors.iter().find(|m| m.name == name)
}

/// Get the primary monitor.
pub fn get_primary_monitor(monitors: &[Monitor]) -> Option<&Monitor> {
    monitors.iter().find(|m| m.primary)
}

/// Capture a specific monitor.
pub fn capture_monitor(
    conn: &Connection,
    shm: &ShmCapture,
    monitor_name: &str,
) -> Result<RegionCaptureResult> {
    let monitors = get_monitors(conn)?;

    let monitor = find_monitor(&monitors, monitor_name)
        .ok_or_else(|| GarshotError::MonitorNotFound(monitor_name.to_string()))?;

    tracing::debug!(
        "Capturing monitor {} ({}x{}+{}+{})",
        monitor.name,
        monitor.width,
        monitor.height,
        monitor.x,
        monitor.y
    );

    capture_region(conn, shm, &monitor.to_region())
}

/// List available monitor names.
pub fn list_monitor_names(conn: &Connection) -> Result<Vec<String>> {
    let monitors = get_monitors(conn)?;
    Ok(monitors.into_iter().map(|m| m.name).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_to_region() {
        let monitor = Monitor {
            name: "HDMI-1".to_string(),
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
            primary: false,
            output_id: 0,
            crtc_id: 0,
        };

        let region = monitor.to_region();
        assert_eq!(region.x, 1920);
        assert_eq!(region.y, 0);
        assert_eq!(region.width, 1920);
        assert_eq!(region.height, 1080);
    }
}
