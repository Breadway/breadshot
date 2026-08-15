mod capture;
mod config;
mod listen;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use capture::{Mode, Overrides};
use config::Config;

#[derive(Parser)]
#[command(
    name = "breadshot",
    version,
    about = "Screenshot utility for the bread ecosystem",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Path to config file
    #[arg(long, value_name = "FILE", global = true)]
    config: Option<PathBuf>,
}

#[derive(Args)]
struct CaptureOpts {
    /// Copy to clipboard only, don't save to disk
    #[arg(long, short = 'c')]
    clipboard_only: bool,

    /// Suppress notifications
    #[arg(long, short = 's')]
    silent: bool,

    /// Freeze screen during selection (requires hyprpicker)
    #[arg(long, short = 'z')]
    freeze: bool,

    /// Freeze the captured frame and annotate (requires satty or swappy)
    #[arg(long, short = 'a')]
    annotate: bool,

    /// Override save directory from config
    #[arg(long, short = 'o', value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Override output filename (without path)
    #[arg(long, short = 'f', value_name = "NAME")]
    filename: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Select a region interactively
    Region(CaptureOpts),
    /// Click to select a window
    Window(CaptureOpts),
    /// Click to select a monitor
    Output(CaptureOpts),
    /// Capture the active window
    #[command(name = "active-window")]
    ActiveWindow(CaptureOpts),
    /// Capture the active monitor
    #[command(name = "active-output")]
    ActiveOutput(CaptureOpts),
    /// Capture a region, freeze the frame, and annotate
    Annotate(CaptureOpts),
    /// Subscribe to bread.command.shot.** and honor region/annotate captures
    Listen,
}

impl Command {
    fn into_capture(self) -> Option<(Mode, CaptureOpts)> {
        match self {
            Self::Region(opts) => Some((Mode::Region, opts)),
            Self::Window(opts) => Some((Mode::Window, opts)),
            Self::Output(opts) => Some((Mode::Output, opts)),
            Self::ActiveWindow(opts) => Some((Mode::ActiveWindow, opts)),
            Self::ActiveOutput(opts) => Some((Mode::ActiveOutput, opts)),
            Self::Annotate(mut opts) => {
                opts.annotate = true;
                Some((Mode::Region, opts))
            }
            Self::Listen => None,
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    let cli = Cli::parse();

    let config = match &cli.config {
        Some(path) => Config::load_from(path)?,
        None => Config::load()?,
    };

    let Some((mode, opts)) = cli.command.into_capture() else {
        return listen::run(&config);
    };

    capture::run(
        &mode,
        &config,
        Overrides {
            clipboard_only: opts.clipboard_only,
            silent: opts.silent,
            freeze: opts.freeze,
            annotate: opts.annotate,
            output_dir: opts.output_dir,
            filename: opts.filename,
        },
    )?;
    Ok(())
}
