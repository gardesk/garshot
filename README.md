# garshot

X11 screenshot utility for the gar desktop suite.

## Features

- Full screen, region, window capture
- Interactive selection with blur overlay
- Multi-monitor support (XRandR)
- Output formats: PNG, JPEG, WebP
- Auto-copy to clipboard
- Optional cursor capture
- MIT-SHM for fast screen capture

## Dependencies

Build:
- Rust 1.85+
- libxcb, libx11
- cairo

Runtime:
- X11
- xclip (for clipboard)
- libnotify (for --notify, optional)

### Fedora/RHEL
```sh
sudo dnf install libxcb-devel libX11-devel cairo-devel xclip libnotify
```

### Debian/Ubuntu
```sh
sudo apt install libxcb1-dev libx11-dev libcairo2-dev xclip libnotify-bin
```

### Arch
```sh
sudo pacman -S libxcb libx11 cairo xclip libnotify
```

## Build

```sh
cargo build --release
```

Binary at `target/release/garshot`.

## Install

```sh
cargo install --path garshot
```

Or copy manually:
```sh
sudo install -Dm755 target/release/garshot /usr/local/bin/garshot
```

## Usage

### Full screen
```sh
garshot screen                     # All monitors
garshot screen -m DP-1             # Specific monitor
garshot screen -c                  # Include cursor
garshot screen -f jpeg -o shot.jpg # JPEG output
garshot screen --delay 3           # 3 second countdown
garshot screen --notify            # Desktop notification on save
garshot screen -o - | feh -        # Pipe to viewer
```

### Interactive selection
```sh
garshot select                     # Blur overlay, drag to select
garshot select -b 20               # Custom blur radius
```

Controls:
- Left click + drag: select region
- Right click / Escape: cancel
- Release: capture

### Window capture
```sh
garshot window                     # Active window
garshot window -i 0x1400007        # By window ID
garshot window -d                  # Include decorations
```

### Region by geometry
```sh
garshot region -g 800x600+100+50   # WxH+X+Y format
```

### List monitors
```sh
garshot monitors
```

### Common options

| Option | Description |
|--------|-------------|
| `-o, --output <PATH>` | Output file path (use `-` for stdout) |
| `-f, --format <FMT>` | png, jpeg, webp (default: png) |
| `-c, --cursor` | Include cursor in capture |
| `--no-clipboard` | Don't copy to clipboard |
| `--delay <SECS>` | Delay before capture (screen/region/window) |
| `--notify` | Show desktop notification on save |

## Configuration

Config file: `~/.config/garshot/config.toml`

```toml
[general]
save_dir = "~/Pictures/Screenshots"
format = "png"
quality = 90
include_cursor = false

[selection]
blur_radius = 15
line_color = "#ff6600"
line_width = 2

[naming]
pattern = "screenshot-%Y%m%d-%H%M%S"
```

## Keybindings (gar)

Add to `~/.config/gar/init.lua`:

```lua
gar.bind("ctrl+shift+4", function()
    gar.exec("garshot select")
end)
gar.bind("ctrl+shift+5", function()
    gar.exec("garshot screen")
end)
gar.bind("ctrl+shift+6", function()
    gar.exec("garshot window")
end)
```

## Environment

- `RUST_LOG=debug` - Enable debug logging

## License

MIT
