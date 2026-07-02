# breadshot

Wayland screenshot utility for the bread ecosystem. Wraps `grim`, `slurp`, and `wl-copy` with Hyprland-aware geometry resolution, clipboard integration, and desktop notifications.

## Requirements

Required (must be in `$PATH`):

- `grim` — Wayland screenshot tool
- `slurp` — region/window selection
- `wl-copy` (wl-clipboard) — clipboard write
- `hyprctl` — Hyprland IPC (for window and monitor geometry)

Optional:

- `hyprpicker` — screen freeze during selection (`--freeze`)
- `notify-send` — desktop notifications (silently skipped if absent)

## Build and install

```sh
# build only
cargo build --release

# build and install to /usr/local/bin
make install

# install to a custom prefix
make install PREFIX=/usr
```

## Usage

```
breadshot <mode> [options]
```

### Modes

| Mode | Description |
|---|---|
| `region` | Interactive region selection |
| `window` | Click to select a window on the active workspaces |
| `output` | Click to select a monitor |
| `active-window` | Capture the currently focused window |
| `active-output` | Capture the monitor containing the active workspace |

### Options

| Flag | Short | Description |
|---|---|---|
| `--clipboard-only` | `-c` | Copy to clipboard only, do not save to disk |
| `--silent` | `-s` | Suppress notifications |
| `--freeze` | `-z` | Freeze screen during selection (requires `hyprpicker`) |
| `--output-dir <DIR>` | `-o` | Override the save directory from config |
| `--filename <NAME>` | `-f` | Override the output filename (without path) |
| `--config <FILE>` | | Use a specific config file |

### Examples

```sh
# interactive region selection, save and copy
breadshot region

# capture active window to clipboard only
breadshot active-window --clipboard-only

# region selection with screen frozen, saved to a custom path
breadshot region --freeze --output-dir ~/Desktop --filename capture.png
```

## Configuration

Config file: `~/.config/breadshot/config.toml`

All keys are optional. Missing keys fall back to the defaults shown below.

```toml
# Directory where screenshots are saved
save_dir = "~/Pictures/Screenshots"

# Suppress notifications globally
silent = false

# Freeze screen during selection by default (requires hyprpicker)
freeze = false

# Notification display duration in milliseconds
notif_timeout = 5000

# strftime format used to generate filenames
# Filename pattern: <date_format>_breadshot.png
date_format = "%Y-%m-%d-%H%M%S"
```

## License

MIT — see [LICENSE](LICENSE).
