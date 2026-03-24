use crate::audio::AudioPlayer;
use crate::config::Config;
use crate::input::InputHandler;
use crate::lyrics::LyricsManager;
use crate::ui::Ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime};

use super::{Mode, PlayMode, SearchMode, SortMode};

pub struct App {
    pub mode: Mode,
    pub running: bool,
    pub config: Config,
    
    // Playlist
    pub songs: Vec<Song>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    
    // Playback
    pub current_song_index: Option<usize>,
    pub is_playing: bool,
    pub current_pos: Duration,
    pub duration: Duration,
    pub play_mode: PlayMode,
    
    // Search
    pub search_query: String,
    pub search_mode: SearchMode,
    pub sort_mode: SortMode,
    
    // Status message
    pub status_message: String,
    pub status_expiry: Option<Instant>,
    
    // Background scanning
    scanning: bool,
    scan_rx: Option<Receiver<ScanMessage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    #[serde(with = "duration_serde")]
    pub duration: Duration,
    /// File modification time for cache invalidation
    pub mtime: u64,
}

/// Helper module for serializing Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Cache structure for storing songs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongsCache {
    /// Music folder path this cache is for
    pub music_folder: String,
    /// List of cached songs
    pub songs: Vec<Song>,
}

/// Messages from background scan thread
#[derive(Debug)]
enum ScanMessage {
    /// Progress update with number of files found
    Progress { found: usize },
    /// Scan completed with results
    Done {
        songs: Vec<Song>,
        cached_count: usize,
        new_count: usize,
        updated_count: usize,
    },
}

impl Default for App {
    fn default() -> Self {
        Self {
            mode: Mode::Normal,
            running: true,
            config: Config::default(),
            songs: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            current_song_index: None,
            is_playing: false,
            current_pos: Duration::ZERO,
            duration: Duration::ZERO,
            play_mode: PlayMode::None,
            search_query: String::new(),
            search_mode: SearchMode::default(),
            sort_mode: SortMode::default(),
            status_message: String::new(),
            status_expiry: None,
            scanning: false,
            scan_rx: None,
        }
    }
}

impl App {
    pub fn new(music_dir: Option<String>) -> Result<Self> {
        let config = Config::load()?;
        let mut app = Self {
            config,
            ..Self::default()
        };
        if let Some(dir) = music_dir {
            app.config.music_folder = dir;
            app.config.save()?;
        }
        // Load cache synchronously for instant display
        let music_dir_str = app.get_music_dir_str();
        if let Some(cache) = app.load_cache(&music_dir_str) {
            app.songs = cache.songs;
            app.filtered_indices = (0..app.songs.len()).collect();
            app.status_message = format!("发现 {} 首歌曲", app.songs.len());
        }
        Ok(app)
    }
    
    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::cursor::Hide
        )?;
        
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        // Main loop
        let res = self.main_loop(&mut terminal);
        
        // Restore terminal
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        )?;
        
        res
    }
    
    fn main_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);
        
        let mut audio_player = AudioPlayer::new()?;
        let mut lyrics_manager = LyricsManager::new();
        
        // Start background scan
        self.start_scan();
        
        while self.running {
            // Poll background scan results
            self.poll_scan();
            
            // Draw UI
            terminal.draw(|f| {
                let ui = Ui::new(self, &lyrics_manager);
                ui.render(f);
            })?;
            
            // Handle events
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    // Only handle key press events (ignore release)
                    if key.kind == KeyEventKind::Press {
                        let handler = InputHandler::new();
                        handler.handle(self, &mut audio_player, key)?;
                    }
                }
            }
            
            // Update current playback position from audio player
            if self.is_playing {
                self.current_pos = audio_player.current_position();
            }
            
            // Check if song finished (was playing but now empty)
            if self.is_playing && self.current_song_index.is_some() && !audio_player.is_playing() {
                self.is_playing = false;
                // Auto-play next song
                self.next_song(&mut audio_player);
            }
            
            // Load and update lyrics
            if let Some(idx) = self.current_song_index {
                if idx < self.songs.len() {
                    let song = &self.songs[idx];
                    lyrics_manager.load(&song.path);
                }
            }
            
            // Tick
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
        
        Ok(())
    }
    
    fn get_music_dir(&self) -> PathBuf {
        if self.config.music_folder.is_empty() {
            dirs::audio_dir().unwrap_or_else(|| PathBuf::from("."))
        } else {
            PathBuf::from(&self.config.music_folder)
        }
    }
    
    fn get_music_dir_str(&self) -> String {
        self.get_music_dir().to_str().unwrap_or(".").to_string()
    }
    
    /// Start background scanning in a separate thread
    pub fn start_scan(&mut self) {
        if self.scanning {
            return;
        }
        
        let music_dir = self.get_music_dir();
        let music_dir_str = self.get_music_dir_str();
        let cache = self.load_cache(&music_dir_str);
        
        let (tx, rx) = mpsc::channel();
        self.scanning = true;
        self.scan_rx = Some(rx);
        self.status_message = "正在扫描媒体库...".to_string();
        self.status_expiry = None;
        
        std::thread::spawn(move || {
            Self::background_scan(music_dir, cache, tx);
        });
    }
    
    /// Poll for background scan results
    fn poll_scan(&mut self) {
        if !self.scanning {
            return;
        }
        let Some(rx) = self.scan_rx.take() else { return };
        
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ScanMessage::Progress { found } => {
                    self.status_message = format!("正在扫描媒体库... 已发现 {} 首", found);
                    self.status_expiry = None;
                }
                ScanMessage::Done { songs, cached_count, new_count, updated_count } => {
                    // Preserve currently playing song
                    let playing_path = self.current_song_index
                        .and_then(|idx| self.songs.get(idx))
                        .map(|s| s.path.clone());
                    
                    self.songs = songs;
                    self.filtered_indices = (0..self.songs.len()).collect();
                    
                    // Restore current song index
                    if let Some(ref path) = playing_path {
                        self.current_song_index = self.songs.iter().position(|s| s.path == *path);
                    }
                    
                    if self.selected_index >= self.filtered_indices.len() {
                        self.selected_index = 0;
                        self.scroll_offset = 0;
                    }
                    
                    self.scanning = false;
                    // rx dropped here (scan_rx stays None)
                    
                    // Save cache
                    let music_dir_str = self.get_music_dir_str();
                    let _ = self.save_cache(&music_dir_str);
                    
                    if new_count > 0 || updated_count > 0 {
                        self.status_message = format!(
                            "扫描完成: {} 首 (缓存: {} | 新增: {} | 更新: {})",
                            self.songs.len(), cached_count, new_count, updated_count
                        );
                    } else {
                        self.status_message = format!("扫描完成: {} 首歌曲", self.songs.len());
                    }
                    return;
                }
            }
        }
        
        // Still scanning, put receiver back
        self.scan_rx = Some(rx);
    }
    
    /// Background scan thread function
    fn background_scan(music_dir: PathBuf, cache: Option<SongsCache>, tx: Sender<ScanMessage>) {
        use rayon::prelude::*;
        use walkdir::WalkDir;
        
        let extensions = ["mp3", "flac", "wav", "ogg", "m4a", "aac"];
        
        // Phase 1: Collect all music file paths and mtimes
        let file_entries: Vec<(String, u64)> = WalkDir::new(&music_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let ext = path.extension()?.to_str()?.to_lowercase();
                if extensions.contains(&ext.as_str()) {
                    let path_str = path.to_str()?.to_string();
                    let mtime = get_file_mtime(path);
                    Some((path_str, mtime))
                } else {
                    None
                }
            })
            .collect();
        
        let _ = tx.send(ScanMessage::Progress { found: file_entries.len() });
        
        // Phase 2: Determine which files need parsing
        let cached_map: HashMap<String, Song> = match &cache {
            Some(c) => c.songs.iter().map(|s| (s.path.clone(), s.clone())).collect(),
            None => HashMap::new(),
        };
        
        let mut to_parse_new = Vec::new();
        let mut to_parse_updated = Vec::new();
        let mut cached_count = 0usize;
        
        for (path, mtime) in &file_entries {
            match cached_map.get(path.as_str()) {
                Some(cached) if cached.mtime == *mtime => {
                    cached_count += 1;
                }
                Some(_) => {
                    to_parse_updated.push(path.clone());
                }
                None => {
                    to_parse_new.push(path.clone());
                }
            }
        }
        
        // Phase 3: Parse new files in parallel using rayon
        let parsed_new: HashMap<String, Song> = to_parse_new
            .par_iter()
            .filter_map(|path| parse_song(path).ok().map(|song| (path.clone(), song)))
            .collect();
        
        // Phase 3b: Parse updated files in parallel
        let parsed_updated: HashMap<String, Song> = to_parse_updated
            .par_iter()
            .filter_map(|path| parse_song(path).ok().map(|song| (path.clone(), song)))
            .collect();
        
        let new_count = parsed_new.len();
        let updated_count = parsed_updated.len();
        
        // Merge parsed results
        let mut parsed_map: HashMap<String, Song> = parsed_new;
        parsed_map.extend(parsed_updated);
        
        // Phase 4: Build final song list preserving directory walk order
        let songs: Vec<Song> = file_entries
            .into_iter()
            .filter_map(|(path, _)| {
                cached_map.get(&path)
                    .cloned()
                    .or_else(|| parsed_map.get(&path).cloned())
            })
            .collect();
        
        let _ = tx.send(ScanMessage::Done {
            songs,
            cached_count,
            new_count,
            updated_count,
        });
    }
    
    /// Get cache file path (unique per music folder)
    fn cache_path(music_folder: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        music_folder.hash(&mut hasher);
        let hash = hasher.finish();
        
        let cache_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cache");
        
        // Create cache directory if not exists
        let _ = std::fs::create_dir_all(&cache_dir);
        
        cache_dir.join(format!("songs_cache_{:016x}.json", hash))
    }
    
    /// Load cache from file
    fn load_cache(&self, music_folder: &str) -> Option<SongsCache> {
        let path = Self::cache_path(music_folder);
        if !path.exists() {
            return None;
        }
        
        let content = std::fs::read_to_string(path).ok()?;
        let cache: SongsCache = serde_json::from_str(&content).ok()?;
        
        // Validate cache is for the same music folder
        if cache.music_folder == music_folder {
            Some(cache)
        } else {
            None
        }
    }
    
    /// Save cache to file
    fn save_cache(&self, music_folder: &str) -> Result<()> {
        let cache = SongsCache {
            music_folder: music_folder.to_string(),
            songs: self.songs.clone(),
        };
        
        let path = Self::cache_path(music_folder);
        let content = serde_json::to_string_pretty(&cache)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    pub fn play_selected(&mut self, audio_player: &mut AudioPlayer) {
        if self.filtered_indices.is_empty() {
            return;
        }
        if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = 0;
        }
        
        let song_idx = self.filtered_indices[self.selected_index];
        if let Some(song) = self.songs.get(song_idx) {
            match audio_player.play(&song.path) {
                Ok(()) => {
                    self.current_song_index = Some(song_idx);
                    self.is_playing = true;
                    self.duration = song.duration;
                    self.current_pos = Duration::ZERO;
                    self.status_message = format!("播放中: {} - {}", song.artist, song.title);
                }
                Err(e) => {
                    self.set_status(format!("播放失败: {}", e));
                }
            }
        }
    }
    
    pub fn toggle_pause(&mut self, audio_player: &mut AudioPlayer) {
        if self.current_song_index.is_some() {
            audio_player.toggle_pause();
            self.is_playing = !self.is_playing;
            self.status_message = if self.is_playing { "播放" } else { "暂停" }.to_string();
        }
    }
    
    pub fn stop(&mut self, audio_player: &mut AudioPlayer) {
        audio_player.stop();
        self.is_playing = false;
        self.current_pos = Duration::ZERO;
        self.status_message = "停止".to_string();
    }
    
    pub fn next_song(&mut self, audio_player: &mut AudioPlayer) {
        if self.filtered_indices.is_empty() {
            return;
        }
        
        // Find current position in filtered list
        let current_filtered = self.current_song_index
            .and_then(|idx| self.filtered_indices.iter().position(|&i| i == idx));
        
        let next = match self.play_mode {
            PlayMode::Single => {
                // Repeat current song
                if let Some(pos) = current_filtered {
                    pos
                } else {
                    0
                }
            }
            PlayMode::Shuffle => {
                // Random next song
                use rand::Rng;
                let mut rng = rand::rng();
                rng.random_range(0..self.filtered_indices.len())
            }
            PlayMode::All | PlayMode::None => {
                if let Some(pos) = current_filtered {
                    if pos + 1 < self.filtered_indices.len() {
                        pos + 1
                    } else if self.play_mode == PlayMode::All {
                        0
                    } else {
                        self.stop(audio_player);
                        return;
                    }
                } else {
                    0
                }
            }
        };
        
        self.selected_index = next;
        self.play_selected(audio_player);
    }
    
    pub fn prev_song(&mut self, audio_player: &mut AudioPlayer) {
        if self.filtered_indices.is_empty() {
            return;
        }
        
        // If current position > 3 seconds, restart current song
        if self.current_pos > Duration::from_secs(3) {
            if let Some(idx) = self.current_song_index {
                if let Some(song) = self.songs.get(idx) {
                    let _ = audio_player.play(&song.path);
                    self.current_pos = Duration::ZERO;
                    return;
                }
            }
        }
        
        let current_filtered = self.current_song_index
            .and_then(|idx| self.filtered_indices.iter().position(|&i| i == idx));
        
        let prev = if let Some(pos) = current_filtered {
            if pos > 0 { pos - 1 } else { self.filtered_indices.len() - 1 }
        } else {
            0
        };
        
        self.selected_index = prev;
        self.play_selected(audio_player);
    }
    
    pub fn move_up(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
            self.adjust_scroll();
        }
    }
    
    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected_index < self.filtered_indices.len() - 1 {
            self.selected_index += 1;
            self.adjust_scroll();
        }
    }
    
    pub fn adjust_scroll(&mut self) {
        let visible_height = 10; // Will be updated dynamically
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index - visible_height + 1;
        }
    }

    pub fn apply_filter(&mut self) {
        self.filtered_indices.clear();

        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.songs.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            for (i, song) in self.songs.iter().enumerate() {
                let matches = match self.search_mode {
                    SearchMode::TitleArtist => {
                        song.title.to_lowercase().contains(&query)
                            || song.artist.to_lowercase().contains(&query)
                    }
                    SearchMode::Artist => {
                        song.artist.to_lowercase().contains(&query)
                    }
                    SearchMode::Album => {
                        song.album.to_lowercase().contains(&query)
                    }
                    SearchMode::Filename => {
                        std::path::Path::new(&song.path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.to_lowercase().contains(&query))
                            .unwrap_or(false)
                    }
                };
                if matches {
                    self.filtered_indices.push(i);
                }
            }
        }

        // Apply current sort order
        self.sort_songs();

        self.selected_index = 0;
        self.scroll_offset = 0;
        self.status_message = format!("匹配到 {} 首 ({})", self.filtered_indices.len(), self.search_mode.as_str());
    }

    pub fn sort_songs(&mut self) {
        let songs = &self.songs;
        let sort_mode = self.sort_mode;

        // Sort filtered_indices by the sort criteria
        self.filtered_indices.sort_by(|&a, &b| {
            let sa = &songs[a];
            let sb = &songs[b];
            match sort_mode {
                SortMode::Filename => {
                    sa.path.cmp(&sb.path)
                }
                SortMode::Artist => {
                    let cmp = sa.artist.to_lowercase().cmp(&sb.artist.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        cmp
                    } else {
                        sa.title.to_lowercase().cmp(&sb.title.to_lowercase())
                    }
                }
                SortMode::Album => {
                    let cmp = sa.album.to_lowercase().cmp(&sb.album.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        cmp
                    } else {
                        sa.title.to_lowercase().cmp(&sb.title.to_lowercase())
                    }
                }
                SortMode::Folder => {
                    let dir_a = std::path::Path::new(&sa.path).parent()
                        .and_then(|p| p.to_str()).unwrap_or("");
                    let dir_b = std::path::Path::new(&sb.path).parent()
                        .and_then(|p| p.to_str()).unwrap_or("");
                    let cmp = dir_a.to_lowercase().cmp(&dir_b.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        cmp
                    } else {
                        sa.title.to_lowercase().cmp(&sb.title.to_lowercase())
                    }
                }
            }
        });
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();

        // Apply filter (which also sorts), but preserve selected song
        let selected_song = self.filtered_indices.get(self.selected_index).copied();
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.songs.len()).collect();
        } else {
            self.apply_filter();
            return; // apply_filter already resets selection and sorts
        }
        self.sort_songs();

        // Try to restore selection to the same song
        if let Some(song_idx) = selected_song {
            if let Some(pos) = self.filtered_indices.iter().position(|&i| i == song_idx) {
                self.selected_index = pos;
                self.adjust_scroll();
            } else {
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
        }

        self.set_status(format!("排序: {}", self.sort_mode.as_str()));
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_expiry = Some(Instant::now() + Duration::from_secs(3));
    }
    
    pub fn quit(&mut self) {
        self.running = false;
    }
}

/// Get file modification time as unix timestamp
fn get_file_mtime(path: &std::path::Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0)
}

/// Parse a song file's metadata
fn parse_song(path: &str) -> Result<Song> {
    use lofty::probe::Probe;
    use lofty::file::{TaggedFileExt, AudioFile};
    use lofty::tag::Accessor;
    
    let file_path = std::path::Path::new(path);
    let tagged_file = Probe::open(file_path)?.read()?;
    let tag = tagged_file.primary_tag();
    let properties = tagged_file.properties();
    
    let duration = properties.duration();
    let mtime = get_file_mtime(file_path);
    
    let (title, artist, album) = if let Some(tag) = tag {
        (
            tag.title()
                .map(|s| s.to_string())
                .unwrap_or_else(|| file_path.file_stem().unwrap().to_str().unwrap_or("Unknown").to_string()),
            tag.artist()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "未知歌手".to_string()),
            tag.album()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "未知专辑".to_string()),
        )
    } else {
        (
            file_path.file_stem().unwrap().to_str().unwrap_or("Unknown").to_string(),
            "未知歌手".to_string(),
            "未知专辑".to_string(),
        )
    };
    
    Ok(Song {
        path: path.to_string(),
        title,
        artist,
        album,
        duration,
        mtime,
    })
}
