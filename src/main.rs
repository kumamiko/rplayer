mod app;
mod audio;
mod config;
mod input;
mod lyrics;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use std::panic;

/// A TUI local music player with lyrics display and Vim-style keybindings
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Music directory to scan
    #[arg(short = 'd', long = "dir")]
    music_dir: Option<String>,
}

fn main() -> Result<()> {
    // Set up panic hook to restore terminal before showing error
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Try to restore terminal state
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        
        // Print the panic message
        eprintln!("\n程序发生错误:");
        original_hook(panic_info);
    }));
    
    let args = Args::parse();
    let mut app = App::new(args.music_dir)?;
    app.run()?;
    Ok(())
}
