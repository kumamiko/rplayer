mod app;
mod audio;
mod config;
mod input;
mod lyrics;
mod ui;

use anyhow::Result;
use app::App;

fn main() -> Result<()> {
    let mut app = App::new()?;
    app.run()?;
    Ok(())
}
