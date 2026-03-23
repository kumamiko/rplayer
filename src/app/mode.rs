/// Input mode for Vim-style keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Normal mode: navigation, playback control
    #[default]
    Normal,
    /// Search mode: typing search query
    Search,
    /// Command mode: typing command after ':'
    Command,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Search => "SEARCH",
            Mode::Command => "COMMAND",
        }
    }
}
