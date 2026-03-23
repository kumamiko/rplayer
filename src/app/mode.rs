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

/// Search field mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    /// Search in title or artist (default)
    #[default]
    TitleArtist,
    /// Search in artist only
    Artist,
    /// Search in album only
    Album,
    /// Search in filename only
    Filename,
}

impl SearchMode {
    /// Cycle to next search mode: TitleArtist -> Artist -> Album -> Filename -> TitleArtist
    pub fn next(&self) -> Self {
        match self {
            SearchMode::TitleArtist => SearchMode::Artist,
            SearchMode::Artist => SearchMode::Album,
            SearchMode::Album => SearchMode::Filename,
            SearchMode::Filename => SearchMode::TitleArtist,
        }
    }
    
    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchMode::TitleArtist => "歌曲/歌手",
            SearchMode::Artist => "歌手",
            SearchMode::Album => "专辑",
            SearchMode::Filename => "文件名",
        }
    }
}

/// Playback mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayMode {
    /// No repeat
    #[default]
    None,
    /// Repeat single song
    Single,
    /// Repeat all songs in list
    All,
    /// Shuffle playback
    Shuffle,
}

impl PlayMode {
    /// Cycle to next play mode: None -> Single -> All -> Shuffle -> None
    pub fn next(&self) -> Self {
        match self {
            PlayMode::None => PlayMode::Single,
            PlayMode::Single => PlayMode::All,
            PlayMode::All => PlayMode::Shuffle,
            PlayMode::Shuffle => PlayMode::None,
        }
    }
    
    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            PlayMode::None => "顺序播放",
            PlayMode::Single => "单曲循环",
            PlayMode::All => "列表循环",
            PlayMode::Shuffle => "随机播放",
        }
    }
    
    /// Get icon
    pub fn icon(&self) -> &'static str {
        match self {
            PlayMode::None => "→",
            PlayMode::Single => "🔂",
            PlayMode::All => "🔁",
            PlayMode::Shuffle => "🔀",
        }
    }
}
