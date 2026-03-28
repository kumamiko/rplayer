use serde::{Deserialize, Serialize};

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
    /// Theme color input mode: typing hex color
    Theme,
    /// Switch cache: select different music folder cache
    SwitchCache,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Normal => "Normal",
            Mode::Search => "Search",
            Mode::ConfirmRefresh => "Confirm",
            Mode::Help => "Help",
            Mode::Theme => "Theme",
            Mode::SwitchCache => "SwitchCache",
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

/// Sorting mode for the playlist
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortMode {
    /// Sort by song title (default)
    #[default]
    Title,
    /// Sort by artist
    Artist,
    /// Sort by album
    Album,
    /// Sort by parent folder
    Folder,
}

impl SortMode {
    /// Cycle to next sort mode: Title -> Artist -> Album -> Folder -> Title
    pub fn next(&self) -> Self {
        match self {
            SortMode::Title => SortMode::Artist,
            SortMode::Artist => SortMode::Album,
            SortMode::Album => SortMode::Folder,
            SortMode::Folder => SortMode::Title,
        }
    }

    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            SortMode::Title => "歌曲名",
            SortMode::Artist => "歌手",
            SortMode::Album => "专辑",
            SortMode::Folder => "文件夹",
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
