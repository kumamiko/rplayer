use crate::audio::AudioPlayer;
use crate::config::Config;
use crate::input::InputHandler;
use crate::lyrics::LyricsManager;
use crate::ui::Ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::{Duration, Instant};

use super::Mode;

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
    
    // Search
    pub search_query: String,
    
    // Status message
    pub status_message: String,
    pub status_expiry: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct Song {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
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
            search_query: String::new(),
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
        // Use command line arg if provided, otherwise use config
        if let Some(dir) = music_dir {
            app.config.music_folder = dir;
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
        use walkdir::WalkDir;
        
        let music_dir = if self.config.music_folder.is_empty() {
            dirs::audio_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
        } else {
            std::path::PathBuf::from(&self.config.music_folder)
        };
        
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
        
        self.status_message = format!("发现 {} 首歌曲", self.songs.len());
        Ok(())
    }
    
    fn parse_song(&self, path: &std::path::Path) -> Result<Song> {
        use lofty::probe::Probe;
        use lofty::file::{TaggedFileExt, AudioFile};
        use lofty::tag::Accessor;
        
        let tagged_file = Probe::open(path)?.read()?;
        let tag = tagged_file.primary_tag();
        let properties = tagged_file.properties();
        
        let duration = properties.duration();
        
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
            if audio_player.play(&song.path).is_ok() {
                self.current_song_index = Some(song_idx);
                self.is_playing = true;
                self.duration = song.duration;
                self.current_pos = Duration::ZERO;
                self.status_message = format!("Playing: {} - {}", song.artist, song.title);
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
        
        let next = if let Some(pos) = current_filtered {
            if pos + 1 < self.filtered_indices.len() {
                pos + 1
            } else if self.config.repeat {
                0
            } else {
                self.stop(audio_player);
                return;
            }
        } else {
            0
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
                if song.title.to_lowercase().contains(&query)
                    || song.artist.to_lowercase().contains(&query)
                    || song.album.to_lowercase().contains(&query)
                {
                    self.filtered_indices.push(i);
                }
            }
        }
        
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.status_message = format!("匹配到 {} 首", self.filtered_indices.len());
    }
    
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_expiry = Some(Instant::now() + Duration::from_secs(3));
    }
    
    pub fn quit(&mut self) {
        self.running = false;
    }
}
