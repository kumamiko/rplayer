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
use std::time::{Duration, Instant, SystemTime};

use super::{Mode, PlayMode, SearchMode};

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
    
    // Status message
    pub status_message: String,
    pub status_expiry: Option<Instant>,
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
            status_message: String::new(),
            status_expiry: None,
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
        // Use command line arg if provided, save to config
        if let Some(dir) = music_dir {
            app.config.music_folder = dir;
            app.config.save()?;
        }
        app.scan_music_folder()?;
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
        
        while self.running {
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
    
    pub fn scan_music_folder(&mut self) -> Result<()> {
        let music_dir = if self.config.music_folder.is_empty() {
            dirs::audio_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
        } else {
            std::path::PathBuf::from(&self.config.music_folder)
        };
        
        let music_dir_str = music_dir.to_str().unwrap_or(".").to_string();
        
        // Try to load cache
        let cache = self.load_cache(&music_dir_str);
        
        if let Some(ref cached) = cache {
            // Incremental update
            self.incremental_scan(music_dir, cached)?;
        } else {
            // Full scan
            self.full_scan(music_dir)?;
        }
        
        // Save cache after scanning
        self.save_cache(&music_dir_str)?;
        
        self.status_message = format!("发现 {} 首歌曲", self.songs.len());
        Ok(())
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
    
    /// Full scan - parse all files
    fn full_scan(&mut self, music_dir: PathBuf) -> Result<()> {
        use walkdir::WalkDir;
        
        let extensions = ["mp3", "flac", "wav", "ogg", "m4a", "aac"];
        
        self.songs.clear();
        self.filtered_indices.clear();
        
        for entry in WalkDir::new(music_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    if let Ok(song) = self.parse_song(path) {
                        self.filtered_indices.push(self.songs.len());
                        self.songs.push(song);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Incremental scan - only scan changed files
    fn incremental_scan(&mut self, music_dir: PathBuf, cached: &SongsCache) -> Result<()> {
        use walkdir::WalkDir;
        
        let extensions = ["mp3", "flac", "wav", "ogg", "m4a", "aac"];
        
        // Build a map from path to cached song for fast lookup
        let cached_map: HashMap<String, &Song> = cached.songs.iter()
            .map(|s| (s.path.clone(), s))
            .collect();
        
        self.songs.clear();
        self.filtered_indices.clear();
        
        let mut new_count = 0;
        let mut updated_count = 0;
        let mut cached_count = 0;
        
        for entry in WalkDir::new(music_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    let path_str = path.to_str().unwrap_or("").to_string();
                    
                    // Get current file mtime
                    let current_mtime = self.get_file_mtime(path);
                    
                    // Check if we have cached data
                    if let Some(&cached_song) = cached_map.get(&path_str) {
                        if cached_song.mtime == current_mtime {
                            // File unchanged, use cached data
                            self.filtered_indices.push(self.songs.len());
                            self.songs.push(cached_song.clone());
                            cached_count += 1;
                            continue;
                        } else {
                            // File modified, re-parse
                            updated_count += 1;
                        }
                    } else {
                        // New file
                        new_count += 1;
                    }
                    
                    // Parse new or modified file
                    if let Ok(song) = self.parse_song(path) {
                        self.filtered_indices.push(self.songs.len());
                        self.songs.push(song);
                    }
                }
            }
        }
        
        if new_count > 0 || updated_count > 0 {
            self.status_message = format!(
                "缓存: {} 首 | 新增: {} | 更新: {}",
                cached_count, new_count, updated_count
            );
        }
        
        Ok(())
    }
    
    /// Get file modification time as unix timestamp
    fn get_file_mtime(&self, path: &std::path::Path) -> u64 {
        path.metadata()
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0)
    }
    
    fn parse_song(&self, path: &std::path::Path) -> Result<Song> {
        use lofty::probe::Probe;
        use lofty::file::{TaggedFileExt, AudioFile};
        use lofty::tag::Accessor;
        
        let tagged_file = Probe::open(path)?.read()?;
        let tag = tagged_file.primary_tag();
        let properties = tagged_file.properties();
        
        let duration = properties.duration();
        let mtime = self.get_file_mtime(path);
        
        let (title, artist, album) = if let Some(tag) = tag {
            (
                tag.title()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| path.file_stem().unwrap().to_str().unwrap_or("Unknown").to_string()),
                tag.artist()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown Artist".to_string()),
                tag.album()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown Album".to_string()),
            )
        } else {
            (
                path.file_stem().unwrap().to_str().unwrap_or("Unknown").to_string(),
                "Unknown Artist".to_string(),
                "Unknown Album".to_string(),
            )
        };
        
        Ok(Song {
            path: path.to_str().unwrap_or("").to_string(),
            title,
            artist,
            album,
            duration,
            mtime,
        })
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
            self.status_message = if self.is_playing { "Resumed" } else { "Paused" }.to_string();
        }
    }
    
    pub fn stop(&mut self, audio_player: &mut AudioPlayer) {
        audio_player.stop();
        self.is_playing = false;
        self.current_pos = Duration::ZERO;
        self.status_message = "Stopped".to_string();
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
                        // Extract filename from path
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
        
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.status_message = format!("匹配到 {} 首 ({})", self.filtered_indices.len(), self.search_mode.as_str());
    }
    
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_expiry = Some(Instant::now() + Duration::from_secs(3));
    }
    
    pub fn quit(&mut self) {
        self.running = false;
    }
}
