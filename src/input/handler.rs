use crate::app::{App, Mode};
use crate::audio::AudioPlayer;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct InputHandler;

impl InputHandler {
    pub fn new() -> Self {
        Self
    }
    
    pub fn handle(&self, app: &mut App, audio_player: &mut AudioPlayer, key: KeyEvent) -> Result<()> {
        match app.mode {
            Mode::Normal => self.handle_normal(app, audio_player, key),
            Mode::Search => self.handle_search(app, key),
            Mode::Command => self.handle_command(app, audio_player, key),
        }
    }
    
    fn handle_normal(&self, app: &mut App, audio_player: &mut AudioPlayer, key: KeyEvent) -> Result<()> {
        match key.code {
            // Quit
            KeyCode::Char('q') => app.quit(),
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => app.quit(),
            
            // Navigation (Vim style)
            KeyCode::Char('j') | KeyCode::Down => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up => app.move_up(),
            KeyCode::Char('h') => {
                // Seek backward 5 seconds
                audio_player.seek_relative(false, 10)?;
                app.set_status("<< -10s");
            }
            KeyCode::Char('l') => {
                // Seek forward 5 seconds
                audio_player.seek_relative(true, 10)?;
                app.set_status(">> +10s");
            }
            KeyCode::Char('g') => {
                app.selected_index = 0;
                app.scroll_offset = 0;
            }
            KeyCode::Char('G') => {
                app.selected_index = app.filtered_indices.len().saturating_sub(1);
                app.scroll_offset = app.selected_index.saturating_sub(5); // Show some context above
            }
            KeyCode::PageDown => {
                let jump = 10.min(app.filtered_indices.len().saturating_sub(app.selected_index + 1));
                app.selected_index += jump;
            }
            KeyCode::PageUp => {
                let jump = 10.min(app.selected_index);
                app.selected_index -= jump;
            }
            
            // Playback
            KeyCode::Enter => app.play_selected(audio_player),
            KeyCode::Char(' ') => app.toggle_pause(audio_player),
            KeyCode::Char('s') => app.stop(audio_player),
            KeyCode::Char('n') => app.next_song(audio_player),
            KeyCode::Char('p') => app.prev_song(audio_player),
            
            // Volume
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let vol = (audio_player.get_volume() + 0.1).min(2.0);
                audio_player.set_volume(vol);
                app.set_status(format!("音量: {:.0}%", vol * 100.0));
            }
            KeyCode::Char('-') => {
                let vol = (audio_player.get_volume() - 0.1).max(0.0);
                audio_player.set_volume(vol);
                app.set_status(format!("音量: {:.0}%", vol * 100.0));
            }
            
            // Search mode
            KeyCode::Char('/') => {
                app.mode = Mode::Search;
                app.search_query.clear();
            }
            
            // Command mode
            KeyCode::Char(':') => {
                app.mode = Mode::Command;
                app.command_buffer.clear();
            }
            
            // Rescan
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => {
                app.scan_music_folder()?;
            }
            
            // Help
            KeyCode::Char('?') => {
                app.set_status("j/k:nav | h/l:seek | Enter:play | Space:pause | /:search | ::cmd | q:quit");
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    fn handle_search(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                // Cancel search and restore all songs
                app.search_query.clear();
                app.filtered_indices = (0..app.songs.len()).collect();
                app.selected_index = 0;
                app.scroll_offset = 0;
                app.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                app.apply_filter();
                app.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {}
        }
        
        // Live filter (only if still in search mode)
        if app.mode == Mode::Search && !app.search_query.is_empty() {
            app.apply_filter();
        }
        
        Ok(())
    }
    
    fn handle_command(&self, app: &mut App, audio_player: &mut AudioPlayer, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                app.mode = Mode::Normal;
                app.command_buffer.clear();
            }
            KeyCode::Enter => {
                self.execute_command(app, audio_player)?;
                app.mode = Mode::Normal;
                app.command_buffer.clear();
            }
            KeyCode::Backspace => {
                app.command_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.command_buffer.push(c);
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn execute_command(&self, app: &mut App, audio_player: &mut AudioPlayer) -> Result<()> {
        let cmd = app.command_buffer.trim();
        
        if cmd.is_empty() {
            return Ok(());
        }
        
        match cmd {
            "q" | "quit" | "exit" => app.quit(),
            "rescan" | "scan" => app.scan_music_folder()?,
            "repeat" | "loop" => {
                app.config.repeat = !app.config.repeat;
                app.set_status(if app.config.repeat { "Repeat: ON" } else { "Repeat: OFF" });
                app.config.save()?;
            }
            cmd if cmd.starts_with("vol ") => {
                if let Ok(vol) = cmd[4..].parse::<f32>() {
                    let vol = (vol / 100.0).clamp(0.0, 2.0);
                    audio_player.set_volume(vol);
                    app.set_status(format!("Volume: {:.0}%", vol * 100.0));
                }
            }
            cmd if cmd.starts_with("folder ") || cmd.starts_with("dir ") => {
                let folder = cmd.split_once(' ').map(|(_, f)| f.trim()).unwrap_or("");
                if !folder.is_empty() {
                    app.config.set_music_folder(folder.to_string())?;
                    app.scan_music_folder()?;
                }
            }
            _ => {
                app.set_status(format!("Unknown command: {}", cmd));
            }
        }
        
        Ok(())
    }
}
