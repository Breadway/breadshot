mod capture;
mod config;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use capture::{Mode, Overrides};
use config::Config;

#[derive(Parser)]
#[command(
    name = "breadshot",
    version,
    about = "Screenshot utility for the bread ecosystem",
    disable_help_subcommand = true,
)]
struct Cli {
    /// Capture mode
    mode: Mode,

    /// Copy to clipboard only, don't save to disk
    #[arg(long, short = 'c')]
    clipboard_only: bool,

    /// Suppress notifications
    #[arg(long, short = 's')]
    silent: bool,

    /// Freeze screen during selection (requires hyprpicker)
    #[arg(long, short = 'z')]
    freeze: bool,

    /// Override save directory from config
    #[arg(long, short = 'o', value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Override output filename (without path)
    #[arg(long, short = 'f', value_name = "NAME")]
    filename: Option<String>,

    /// Path to config file
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
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

    capture::run(
        &cli.mode,
        &config,
        Overrides {
            clipboard_only: cli.clipboard_only,
            silent: cli.silent,
            freeze: cli.freeze,
            output_dir: cli.output_dir,
            filename: cli.filename,
        },
    )
}
