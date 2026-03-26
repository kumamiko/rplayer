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
    pub playlist_visible_height: usize,
    
    // Playback
    pub current_song_index: Option<usize>,
    pub is_playing: bool,
    pub current_pos: Duration,
    pub duration: Duration,
    pub play_mode: PlayMode,
    
    // Search
    pub search_query: String,
    pub search_cursor: usize,
    pub search_mode: SearchMode,
    pub sort_mode: SortMode,
    
    // Theme color input
    pub theme_color_input: String,
    pub theme_color_cursor: usize,
    
    // Status message
    pub status_message: String,
    pub status_expiry: Option<Instant>,

    // Count prefix (vim-style, e.g. 5j, 10g)
    pub count: Option<usize>,
    
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
            playlist_visible_height: 10,
            current_song_index: None,
            is_playing: false,
            current_pos: Duration::ZERO,
            duration: Duration::ZERO,
            play_mode: PlayMode::None,
            search_query: String::new(),
            search_cursor: 0,
            search_mode: SearchMode::default(),
            sort_mode: SortMode::default(),
            theme_color_input: String::new(),
            theme_color_cursor: 0,
            status_message: String::new(),
            status_expiry: None,
            count: None,
            scanning: false,
            scan_rx: None,
        }
    }
}

impl App {
    pub fn new(music_dir: Option<String>) -> Result<Self> {
        let config = Config::load()?;
        let sort_mode = config.sort_mode;
        let mut app = Self {
            config,
            sort_mode,
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
            // Apply saved sort mode
            app.sort_songs();
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

        // Restore last playback state
        let _restored = self.restore_playback_state(&mut audio_player);
        if _restored {
            self.set_status("已恢复上次播放");
        }
        
        while self.running {
            // Poll background scan results
            self.poll_scan();
            
            // Draw UI
            terminal.draw(|f| {
                let mut ui = Ui::new(self, &lyrics_manager);
                ui.render(f);
            })?;
            
            // Handle events
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    // Only handle key press events (ignore release)
                    if key.kind == KeyEventKind::Press {
                        let handler = InputHandler::new();
                        handler.handle(self, &mut audio_player, &mut lyrics_manager, key)?;
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
                self.next_song(&mut audio_player, &mut lyrics_manager);
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

    /// Parse hex color string (e.g. "56B6C2" or "#56B6C2") to (r, g, b)
    fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim().trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b))
    }

    /// Parse theme color from hex string (e.g. "56B6C2" or "#56B6C2")
    /// Returns None if invalid or empty, falling back to default colors
    pub fn theme_color(&self) -> Option<ratatui::style::Color> {
        let (r, g, b) = Self::parse_hex_color(&self.config.themecolor)?;
        Some(ratatui::style::Color::Rgb(r, g, b))
    }

    /// Get a brightened version of theme color for title text.
    /// Boosts saturation by at least 30% and lightness by at least 30% in HSL space.
    pub fn theme_color_bright(&self) -> Option<ratatui::style::Color> {
        let (r, g, b) = Self::parse_hex_color(&self.config.themecolor)?;

        // RGB -> HSL
        let rf = r as f32 / 255.0;
        let gf = g as f32 / 255.0;
        let bf = b as f32 / 255.0;
        let max = rf.max(gf).max(bf);
        let min = rf.min(gf).min(bf);
        let l = (max + min) / 2.0;

        let (h, mut s) = if (max - min).abs() < f32::EPSILON {
            (0.0, 0.0)
        } else {
            let d = max - min;
            let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
            let h = if (max - rf).abs() < f32::EPSILON {
                (gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }
            } else if (max - gf).abs() < f32::EPSILON {
                (bf - rf) / d + 2.0
            } else {
                (rf - gf) / d + 4.0
            };
            (h / 6.0, s)
        };

        // Boost saturation by at least 30%
        s = (s * 1.3).min(1.0).max(if s > 0.0 { 0.3 } else { 0.0 });
        // Boost lightness by at least 50%
        let l = (l * 1.5).min(0.9);

        // HSL -> RGB
        fn hue2rgb(p: f32, q: f32, t: f32) -> f32 {
            let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
            if t < 1.0 / 6.0 { p + (q - p) * 6.0 * t }
            else if t < 0.5 { q }
            else if t < 2.0 / 3.0 { p + (q - p) * (2.0 / 3.0 - t) * 6.0 }
            else { p }
        }

        let (rr, gg, bb) = if s < f32::EPSILON {
            let v = (l * 255.0).min(255.0) as u8;
            (v, v, v)
        } else {
            let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
            let p = 2.0 * l - q;
            (
                (hue2rgb(p, q, h + 1.0 / 3.0) * 255.0).min(255.0) as u8,
                (hue2rgb(p, q, h) * 255.0).min(255.0) as u8,
                (hue2rgb(p, q, h - 1.0 / 3.0) * 255.0).min(255.0) as u8,
            )
        };

        Some(ratatui::style::Color::Rgb(rr, gg, bb))
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
                    // Preserve currently playing song and selected song
                    let playing_path = self.current_song_index
                        .and_then(|idx| self.songs.get(idx))
                        .map(|s| s.path.clone());
                    let selected_path = self.filtered_indices.get(self.selected_index)
                        .and_then(|&idx| self.songs.get(idx))
                        .map(|s| s.path.clone());
                    
                    self.songs = songs;
                    self.filtered_indices = (0..self.songs.len()).collect();
                    
                    // Apply saved sort mode
                    self.sort_songs();
                    
                    // Restore current song index
                    if let Some(ref path) = playing_path {
                        self.current_song_index = self.songs.iter().position(|s| s.path == *path);
                    }
                    
                    // Restore selected index
                    if let Some(ref path) = selected_path {
                        if let Some(song_idx) = self.songs.iter().position(|s| s.path == *path) {
                            if let Some(list_pos) = self.filtered_indices.iter().position(|&i| i == song_idx) {
                                self.selected_index = list_pos;
                                self.adjust_scroll();
                            }
                        }
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
        
        let extensions = ["mp3", "flac", "wav", "ogg", "aac"];
        
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
    /// Windows: <exe_dir>/cache/
    /// Other platforms: ~/.rplayer/cache/
    fn cache_path(music_folder: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        music_folder.hash(&mut hasher);
        let hash = hasher.finish();

        #[cfg(not(target_os = "windows"))]
        let cache_dir = {
            let dir = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".rplayer")
                .join("cache");
            let _ = std::fs::create_dir_all(&dir);
            dir
        };

        #[cfg(target_os = "windows")]
        let cache_dir = {
            let dir = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
                .join("cache");
            let _ = std::fs::create_dir_all(&dir);
            dir
        };

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
    
    pub fn play_selected(&mut self, audio_player: &mut AudioPlayer, lyrics_manager: &mut LyricsManager) {
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
                    lyrics_manager.load(&song.path);
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
    
    pub fn next_song(&mut self, audio_player: &mut AudioPlayer, lyrics_manager: &mut LyricsManager) {
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
        self.play_selected(audio_player, lyrics_manager);
    }
    
    pub fn prev_song(&mut self, audio_player: &mut AudioPlayer, lyrics_manager: &mut LyricsManager) {
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
        self.play_selected(audio_player, lyrics_manager);
    }
    
    pub fn move_up(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
            self.adjust_scroll();
        }
    }

    pub fn move_up_by(&mut self, count: usize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let jump = count.min(self.selected_index);
        self.selected_index -= jump;
        self.adjust_scroll();
    }

    pub fn move_down_by(&mut self, count: usize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let jump = count.min(self.filtered_indices.len().saturating_sub(self.selected_index + 1));
        self.selected_index += jump;
        self.adjust_scroll();
    }

    pub fn goto_line(&mut self, line: usize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let target = (line - 1).min(self.filtered_indices.len() - 1);
        self.selected_index = target;
        self.adjust_scroll();
    }

    pub fn consume_count(&mut self) -> usize {
        let count = self.count.take().unwrap_or(1);
        if count > 1 || self.status_expiry.is_none() {
            self.status_message.clear();
        }
        count
    }
    
    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected_index < self.filtered_indices.len() - 1 {
            self.selected_index += 1;
            self.adjust_scroll();
        }
    }
    
    pub fn adjust_scroll(&mut self) {
        let visible_height = self.playlist_visible_height.max(1);
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
                SortMode::Title => {
                    let cmp = sa.title.to_lowercase().cmp(&sb.title.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        cmp
                    } else {
                        sa.artist.to_lowercase().cmp(&sb.artist.to_lowercase())
                    }
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
    
    pub fn quit(&mut self, audio_player: &AudioPlayer) {
        self.running = false;
        // Save playback state
        self.save_playback_state(audio_player);
    }

    /// Save current playback position to config for restore on next launch
    fn save_playback_state(&mut self, audio_player: &AudioPlayer) {
        // Always save sort mode
        self.config.sort_mode = self.sort_mode;
        
        if let Some(song_idx) = self.current_song_index {
            if let Some(song) = self.songs.get(song_idx) {
                let pos = audio_player.current_position();
                self.config.last_song_path = song.path.clone();
                self.config.last_position_secs = pos.as_secs();
            }
        } else {
            // Not playing anything, clear saved state
            self.config.last_song_path.clear();
            self.config.last_position_secs = 0;
        }
        let _ = self.config.save();
    }

    /// Try to restore last playback state. Returns true if restored.
    pub fn restore_playback_state(&mut self, audio_player: &mut AudioPlayer) -> bool {
        if self.config.last_song_path.is_empty() {
            return false;
        }

        // Find the song in current list
        let song_idx = match self.songs.iter().position(|s| s.path == self.config.last_song_path) {
            Some(idx) => idx,
            None => {
                // Song no longer exists, clear saved state
                self.config.last_song_path.clear();
                self.config.last_position_secs = 0;
                let _ = self.config.save();
                return false;
            }
        };

        let song = &self.songs[song_idx];
        let pos_secs = self.config.last_position_secs;

        if let Ok(()) = audio_player.play(&song.path) {
            self.current_song_index = Some(song_idx);
            self.is_playing = false;
            self.duration = song.duration;
            // Seek to saved position if needed
            if pos_secs > 0 {
                self.current_pos = Duration::from_secs(pos_secs);
                let _ = audio_player.seek_to(&song.path, Duration::from_secs(pos_secs));
            } else {
                self.current_pos = Duration::ZERO;
            }
            audio_player.toggle_pause();
            if let Some(list_pos) = self.filtered_indices.iter().position(|&i| i == song_idx) {
                self.selected_index = list_pos;
                self.adjust_scroll();
            }
            return true;
        }

        false
    }
}

/// Get file modification time as unix timestamp
fn get_file_mtime(path: &std::path::Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0)
}

/// Parse a song file's metadata using Symphonia
fn parse_song(path: &str) -> Result<Song> {
    use symphonia::core::meta::{MetadataOptions, StandardTagKey, Value};
    use symphonia::core::probe::Hint;
    use symphonia::default::get_probe;

    let file_path = std::path::Path::new(path);
    let mtime = get_file_mtime(file_path);
    let stem = file_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let file = std::fs::File::open(file_path)?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = get_probe()
        .format(&hint, mss, &Default::default(), &MetadataOptions::default())?;
    let mut format_reader = probed.format;

    // Get duration from track
    let duration = format_reader.tracks().iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .and_then(|track| {
            track.codec_params.time_base
                .zip(track.codec_params.n_frames)
                .map(|(tb, frames)| {
                    let time = tb.calc_time(frames);
                    std::time::Duration::from_secs_f64(time.seconds as f64 + f64::from(time.frac))
                })
        })
        .unwrap_or(std::time::Duration::ZERO);

    // Collect tags from all metadata sources (upsert merge)
    let mut title = None;
    let mut artist = None;
    let mut album = None;

    // Helper to extract string value from tag
    let get_string = |tag: &symphonia::core::meta::Tag| -> Option<String> {
        match &tag.value {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    };

    // 1. Read probed metadata (e.g., ID3v2 tags in MP3 files - stored BEFORE container)
    if let Some(probed_meta) = probed.metadata.get() {
        if let Some(rev) = probed_meta.current() {
            for tag in rev.tags() {
                if let Some(std_key) = tag.std_key {
                    match std_key {
                        StandardTagKey::TrackTitle if title.is_none() => title = get_string(tag),
                        StandardTagKey::Artist if artist.is_none() => artist = get_string(tag),
                        StandardTagKey::Album if album.is_none() => album = get_string(tag),
                        _ => {}
                    }
                }
            }
        }
    }

    // 2. Read container metadata (e.g., Vorbis Comments in FLAC, iTunes metadata in M4A)
    let mut container_meta = format_reader.metadata();
    while !container_meta.is_latest() {
        container_meta.pop();
    }
    if let Some(rev) = container_meta.current() {
        for tag in rev.tags() {
            if let Some(std_key) = tag.std_key {
                match std_key {
                    StandardTagKey::TrackTitle if title.is_none() => title = get_string(tag),
                    StandardTagKey::Artist if artist.is_none() => artist = get_string(tag),
                    StandardTagKey::Album if album.is_none() => album = get_string(tag),
                    _ => {}
                }
            }
        }
    }

    Ok(Song {
        path: path.to_string(),
        title: title.unwrap_or_else(|| stem),
        artist: artist.unwrap_or_else(|| "未知歌手".to_string()),
        album: album.unwrap_or_else(|| "未知专辑".to_string()),
        duration,
        mtime,
    })
}
