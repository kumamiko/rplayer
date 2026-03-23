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
            Mode::ConfirmRefresh => self.handle_confirm_refresh(app, audio_player, key),
            Mode::Help => self.handle_help(app, key),
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
                // Seek backward 10 seconds
                audio_player.seek_relative(false, 10)?;
                app.set_status("⏪ -10秒");
            }
            KeyCode::Char('l') => {
                // Seek forward 10 seconds
                audio_player.seek_relative(true, 10)?;
                app.set_status("⏩ +10秒");
            }
            KeyCode::Char('g') => {
                app.selected_index = 0;
                app.scroll_offset = 0;
            }
            KeyCode::Char('G') => {
                app.selected_index = app.filtered_indices.len().saturating_sub(1);
                app.scroll_offset = app.selected_index.saturating_sub(5);
            }
            KeyCode::PageDown | KeyCode::Right => {
                let jump = 10.min(app.filtered_indices.len().saturating_sub(app.selected_index + 1));
                app.selected_index += jump;
                app.adjust_scroll();
            }
            KeyCode::PageUp | KeyCode::Left => {
                let jump = 10.min(app.selected_index);
                app.selected_index -= jump;
                app.adjust_scroll();
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
            KeyCode::Char('/') | KeyCode::Char('f') => {
                app.mode = Mode::Search;
                app.search_query.clear();
            }
            
            // Toggle repeat mode
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => {
                app.repeat = !app.repeat;
                let status = if app.repeat { "循环: 开" } else { "循环: 关" };
                app.set_status(status);
            }
            
            // Rescan - enter confirm mode
            KeyCode::Char('R') => {
                app.mode = Mode::ConfirmRefresh;
                app.set_status("确认重新扫描媒体库？ (y/n)");
            }
            
            // Help
            KeyCode::Char('?') => {
                app.mode = Mode::Help;
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
    
    fn handle_confirm_refresh(&self, app: &mut App, _audio_player: &mut AudioPlayer, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.mode = Mode::Normal;
                app.scan_music_folder()?;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.mode = Mode::Normal;
                app.set_status("已取消");
            }
            _ => {}
        }
        Ok(())
    }
    
    fn handle_help(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                app.mode = Mode::Normal;
            }
            _ => {}
        }
        Ok(())
    }
}
