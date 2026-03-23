/// Input mode for Vim-style keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Normal mode: navigation, playback control
    #[default]
    Normal,
    /// Search mode: typing search query
    Search,
    /// Confirm dialog: waiting for user confirmation
    ConfirmRefresh,
    /// Help dialog: showing keybindings
    Help,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Normal => "Normal",
            Mode::Search => "Search",
            Mode::ConfirmRefresh => "Confirm",
            Mode::Help => "Help",
        }
    }
}
