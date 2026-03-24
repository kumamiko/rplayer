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
        // Accumulate digit prefix (vim count)
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && c != '0' || app.count.is_some() && c.is_ascii_digit() {
                let n = c.to_digit(10).unwrap() as usize;
                app.count = Some(app.count.unwrap_or(0) * 10 + n);
                app.status_message = format!("{}", app.count.unwrap());
                app.status_expiry = None;
                return Ok(());
            }
        }

        // Digit 0 with no preceding count acts as... ignore (or could be mapped later)
        // Esc cancels count
        if key.code == KeyCode::Esc {
            if app.count.is_some() {
                app.count = None;
                app.status_message.clear();
                return Ok(());
            }
        }

        match key.code {
            // Quit
            KeyCode::Char('q') => app.quit(),
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => app.quit(),

            // Navigation (Vim style with count)
            KeyCode::Char('j') | KeyCode::Down => {
                let count = app.consume_count();
                if count > 1 {
                    app.move_down_by(count);
                } else {
                    app.move_down();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = app.consume_count();
                if count > 1 {
                    app.move_up_by(count);
                } else {
                    app.move_up();
                }
            }
            KeyCode::Char('h') => {
                app.count = None;
                audio_player.seek_relative(false, 10)?;
                app.set_status("⏪ -10秒");
            }
            KeyCode::Char('l') => {
                app.count = None;
                audio_player.seek_relative(true, 10)?;
                app.set_status("⏩ +10秒");
            }
            KeyCode::Char('g') => {
                let count = app.consume_count();
                if count > 1 {
                    app.goto_line(count);
                } else {
                    app.selected_index = 0;
                    app.scroll_offset = 0;
                }
            }
            KeyCode::Char('G') => {
                app.count = None;
                app.selected_index = app.filtered_indices.len().saturating_sub(1);
                app.scroll_offset = app.selected_index.saturating_sub(app.playlist_visible_height.saturating_sub(1));
            }
            KeyCode::PageDown | KeyCode::Right | KeyCode::Char('d') => {
                let visible = app.playlist_visible_height.max(1);
                let remaining = app.filtered_indices.len().saturating_sub(app.selected_index + 1);
                let jump = visible.min(remaining);
                if jump > 0 {
                    let relative = app.selected_index - app.scroll_offset;
                    app.selected_index += jump;
                    app.scroll_offset = app.selected_index.saturating_sub(relative);
                    let max_scroll = app.filtered_indices.len().saturating_sub(visible);
                    app.scroll_offset = app.scroll_offset.min(max_scroll);
                }
            }
            KeyCode::PageUp | KeyCode::Left | KeyCode::Char('u') => {
                let visible = app.playlist_visible_height.max(1);
                let jump = visible.min(app.selected_index);
                if jump > 0 {
                    let relative = app.selected_index - app.scroll_offset;
                    app.selected_index -= jump;
                    app.scroll_offset = app.selected_index.saturating_sub(relative);
                }
            }
            KeyCode::Char('`') | KeyCode::Char('\'') => {
                if let Some(song_idx) = app.current_song_index {
                    if let Some(pos) = app.filtered_indices.iter().position(|&i| i == song_idx) {
                        app.selected_index = pos;
                        app.adjust_scroll();
                    } else {
                        app.set_status("当前歌曲不在显示列表中");
                    }
                } else {
                    app.set_status("没有正在播放的歌曲");
                }
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
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                app.search_mode = app.search_mode.next();
                app.set_status(format!("搜索字段: {}", app.search_mode.as_str()));
            }
            KeyCode::Char('/') | KeyCode::Char('f') => {
                app.mode = Mode::Search;
                app.search_query.clear();
            }
            
            // Clear filter
            KeyCode::Char('F') => {
                app.search_query.clear();
                app.filtered_indices = (0..app.songs.len()).collect();
                app.selected_index = 0;
                app.scroll_offset = 0;
                app.set_status("已清除过滤");
            }
            
            // Toggle play mode
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => {
                app.play_mode = app.play_mode.next();
                app.set_status(format!("{} {}", app.play_mode.icon(), app.play_mode.as_str()));
            }

            // Toggle sort mode
            KeyCode::Char('t') => {
                app.cycle_sort();
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
            
            _ => {
                app.count = None;
            }
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
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                app.search_mode = app.search_mode.next();
                app.set_status(format!("搜索字段: {}", app.search_mode.as_str()));
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {
                app.count = None;
            }
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
                app.start_scan();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.mode = Mode::Normal;
                app.set_status("已取消");
            }
            _ => {
                app.count = None;
            }
        }
        Ok(())
    }
    
    fn handle_help(&self, app: &mut App, _key: KeyEvent) -> Result<()> {
        // Any key closes help
        app.mode = Mode::Normal;
        Ok(())
    }
}
