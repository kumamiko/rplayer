mod app;
mod audio;
mod config;
mod input;
mod lyrics;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;

/// A TUI local music player with lyrics display and Vim-style keybindings
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Music directory to scan
    #[arg(short = 'd', long = "dir")]
    music_dir: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = App::new(args.music_dir)?;
    app.run()?;
    Ok(())
}
