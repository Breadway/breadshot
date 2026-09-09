# breadshot

Wayland screenshot **orchestrator** for the bread ecosystem, not a GUI editor.
It shells out to `grim`, `slurp`, and `wl-copy` with Hyprland-aware geometry
resolution, clipboard integration, and desktop notifications.

This is a different job from `bread-screenshots` (the crate in
`bread-ecosystem`): that one is a capture harness for screenshotting sibling
apps in CI. Do not merge the two.

On BOS, Hyprland binds `Super+Shift+S` / `C` / `P` to breadshot (not grimblast):

| Bind | Action | Command |
|------|--------|---------|
| `Super+Shift+S` | Region → file (and clipboard) | `breadshot region` |
| `Super+Shift+C` | Region → clipboard only | `breadshot region --clipboard-only` |
| `Super+Shift+P` | Screen → file | `breadshot active-output` |

## Requirements

Required (must be in `$PATH`):

- `grim`: Wayland screenshot tool
- `slurp`: region/window selection
- `wl-copy` (wl-clipboard): clipboard write
- `hyprctl`: Hyprland IPC (for window and monitor geometry)

Optional:

- `hyprpicker`: screen freeze during selection (`--freeze`)
- `satty`: freeze the captured frame and annotate (arrows/text/rect). Preferred for `--annotate`
- `swappy`: fallback annotator if `satty` is missing
- `notify-send`: desktop notifications (silently skipped if absent)

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
breadshot annotate [options]
breadshot listen
```

`breadshot listen` is the long-running process that honors
`bread.command.shot.region` (clipboard-only region capture) and
`bread.command.shot.annotate` (region capture, then freeze-and-annotate)
on the bread event bus. See [EVENTS.md](EVENTS.md). Without it, the CLI
still works; bus commands are a silent no-op.

### Modes

| Mode | Description |
|---|---|
| `region` | Interactive region selection |
| `window` | Click to select a window on the active workspaces |
| `output` | Click to select a monitor |
| `active-window` | Capture the currently focused window |
| `active-output` | Capture the monitor containing the active workspace |
| `annotate` | Region capture, then freeze the frame for arrows/text/rect (requires `satty` or `swappy`) |

### Options

| Flag | Short | Description |
|---|---|---|
| `--clipboard-only` | `-c` | Copy to clipboard only, do not save to disk |
| `--silent` | `-s` | Suppress notifications |
| `--freeze` | `-z` | Freeze screen during selection (requires `hyprpicker`) |
| `--annotate` | `-a` | Freeze the captured frame and annotate (requires `satty` or `swappy`) |
| `--output-dir <DIR>` | `-o` | Override the save directory from config |
| `--filename <NAME>` | `-f` | Override the output filename (without path) |
| `--config <FILE>` | | Use a specific config file |

If `--annotate` is set (or `breadshot annotate` is used) but neither
`satty` nor `swappy` is in `$PATH`, breadshot prints a warning and
falls back to the normal grim+slurp capture.

### Examples

```sh
# interactive region selection, save and copy
breadshot region

# capture active window to clipboard only
breadshot active-window --clipboard-only

# region selection with screen frozen, saved to a custom path
breadshot region --freeze --output-dir ~/Desktop --filename capture.png

# region capture, then freeze the frame and annotate
breadshot annotate
breadshot region --annotate
breadshot output --annotate
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

# Freeze the captured frame and annotate by default (requires satty or swappy)
annotate = false

# Notification display duration in milliseconds
notif_timeout = 5000

# strftime format used to generate filenames
# Filename pattern: <date_format>_breadshot.png
date_format = "%Y-%m-%d-%H%M%S"
```

## License

MIT. See [LICENSE](LICENSE).
