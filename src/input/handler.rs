use crate::app::{App, Mode};
use crate::audio::AudioPlayer;
use crate::lyrics::LyricsManager;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct InputHandler;

impl InputHandler {
    pub fn new() -> Self {
        Self
    }
    
    pub fn handle(&self, app: &mut App, audio_player: &mut AudioPlayer, lyrics_manager: &mut LyricsManager, key: KeyEvent) -> Result<()> {
        match app.mode {
            Mode::Normal => self.handle_normal(app, audio_player, lyrics_manager, key),
            Mode::Search => self.handle_search(app, key),
            Mode::ConfirmRefresh => self.handle_confirm_refresh(app, audio_player, lyrics_manager, key),
            Mode::Help => self.handle_help(app, key),
            Mode::Theme => self.handle_theme_color(app, key),
            Mode::SwitchCache => self.handle_switch_cache(app, audio_player, lyrics_manager, key),
        }
    }
    
    fn handle_normal(&self, app: &mut App, audio_player: &mut AudioPlayer, lyrics_manager: &mut LyricsManager, key: KeyEvent) -> Result<()> {
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

        // Backspace: delete last digit from count
        if key.code == KeyCode::Backspace {
            if app.count.is_some() {
                app.count = match app.count.unwrap() / 10 {
                    0 => None,
                    v => Some(v),
                };
                if let Some(c) = app.count {
                    app.status_message = format!("{}", c);
                } else {
                    app.status_message.clear();
                }
                app.status_expiry = None;
                return Ok(());
            }
        }

        match key.code {
            // Quit
            KeyCode::Char('q') => app.quit(audio_player),
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => app.quit(audio_player),

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
                app.set_status("<< -10秒");
            }
            KeyCode::Char('l') => {
                app.count = None;
                audio_player.seek_relative(true, 10)?;
                app.set_status(">> +10秒");
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
                let count = app.consume_count();
                let visible = app.playlist_visible_height.max(1);
                let remaining = app.filtered_indices.len().saturating_sub(app.selected_index + 1);
                let jump = (visible * count).min(remaining);
                if jump > 0 {
                    let relative = app.selected_index - app.scroll_offset;
                    app.selected_index += jump;
                    app.scroll_offset = app.selected_index.saturating_sub(relative);
                    let max_scroll = app.filtered_indices.len().saturating_sub(visible);
                    app.scroll_offset = app.scroll_offset.min(max_scroll);
                }
            }
            KeyCode::PageUp | KeyCode::Left | KeyCode::Char('u') => {
                let count = app.consume_count();
                let visible = app.playlist_visible_height.max(1);
                let jump = (visible * count).min(app.selected_index);
                if jump > 0 {
                    let relative = app.selected_index - app.scroll_offset;
                    app.selected_index -= jump;
                    app.scroll_offset = app.selected_index.saturating_sub(relative);
                }
            }
            KeyCode::Char('`') | KeyCode::Char('\'') => {
                if app.current_song_index.is_none() {
                    app.set_status("没有正在播放的歌曲");
                } else {
                    let prev = app.selected_index;
                    app.scroll_to_playing();
                    if app.selected_index == prev && !app.filtered_indices.is_empty() {
                        app.set_status("当前歌曲不在显示列表中");
                    }
                }
            }

            // Playback
            KeyCode::Enter => {
                if let Some(count) = app.consume_count_optional() {
                    let total = app.filtered_indices.len();
                    if count > total {
                        app.set_status(format!("超出范围 (共{}首)", total));
                        return Ok(());
                    }
                    app.goto_line(count);
                }
                app.play_selected(audio_player, lyrics_manager);
            }
            KeyCode::Char(' ') => app.toggle_pause(audio_player)?,
            KeyCode::Char('n') => app.next_song(audio_player, lyrics_manager),
            KeyCode::Char('p') => app.prev_song(audio_player, lyrics_manager),
            
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
                app.search_cursor = 0;
            }
            
            // Clear filter
            KeyCode::Char('F') => {
                app.search_query.clear();
                app.search_cursor = 0;
                app.filtered_indices = (0..app.songs.len()).collect();
                if !app.scroll_to_playing() {
                    app.selected_index = 0;
                    app.scroll_offset = 0;
                }
                app.set_status("已清除搜索");
            }
            
            // Toggle play mode
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => {
                app.play_mode = app.play_mode.next();
                app.set_status(app.play_mode.as_str().to_string());
            }

            // Toggle sort mode
            KeyCode::Char('t') => {
                app.cycle_sort();
            }
            
            // Theme color input
            KeyCode::Char('T') => {
                app.theme_color_input = app.config.themecolor.clone();
                app.theme_color_cursor = app.theme_color_input.chars().count();
                app.mode = Mode::Theme;
                app.set_status("输入主题色 (6位十六进制, 如 56B6C2)");
                app.status_expiry = None;
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
            
            // Switch cache
            KeyCode::Char('S') => {
                app.load_cached_folders();
                if app.cached_folders.is_empty() {
                    app.set_status("没有可用的缓存");
                } else {
                    app.mode = Mode::SwitchCache;
                }
            }
            
            _ => {}
        }

        // Clear count and its status display for any non-count key
        if app.count.is_some() && app.status_expiry.is_none() {
            app.count = None;
            app.status_message.clear();
        }

        Ok(())
    }
    
    fn handle_search(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                // Cancel search and restore all songs
                app.search_query.clear();
                app.search_cursor = 0;
                app.filtered_indices = (0..app.songs.len()).collect();
                if !app.scroll_to_playing() {
                    app.selected_index = 0;
                    app.scroll_offset = 0;
                }
                app.mode = Mode::Normal;
                app.status_message.clear();
                app.status_expiry = None;
            }
            KeyCode::Enter => {
                app.apply_filter();
                app.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                if app.search_cursor > 0 {
                    let mut chars: Vec<char> = app.search_query.chars().collect();
                    chars.remove(app.search_cursor - 1);
                    app.search_query = chars.iter().collect();
                    app.search_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if app.search_cursor < app.search_query.chars().count() {
                    let mut chars: Vec<char> = app.search_query.chars().collect();
                    chars.remove(app.search_cursor);
                    app.search_query = chars.iter().collect();
                }
            }
            KeyCode::Left => {
                if app.search_cursor > 0 {
                    app.search_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if app.search_cursor < app.search_query.chars().count() {
                    app.search_cursor += 1;
                }
            }
            KeyCode::Home => {
                app.search_cursor = 0;
            }
            KeyCode::End => {
                app.search_cursor = app.search_query.chars().count();
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                app.search_mode = app.search_mode.next();
                app.set_status(format!("搜索字段: {}", app.search_mode.as_str()));
            }
            KeyCode::Char(c) => {
                let mut chars: Vec<char> = app.search_query.chars().collect();
                chars.insert(app.search_cursor, c);
                app.search_query = chars.iter().collect();
                app.search_cursor += 1;
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
    
    fn handle_confirm_refresh(&self, app: &mut App, _audio_player: &mut AudioPlayer, _lyrics_manager: &mut LyricsManager, key: KeyEvent) -> Result<()> {
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
    
    fn handle_theme_color(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                app.theme_color_input.clear();
                app.theme_color_cursor = 0;
                app.mode = Mode::Normal;
                app.status_message.clear();
                app.status_expiry = None;
            }
            KeyCode::Enter => {
                let hex = app.theme_color_input.trim().trim_start_matches('#');
                if hex.is_empty() {
                    app.config.themecolor.clear();
                    app.config.save()?;
                    app.set_status("主题色已恢复默认");
                } else if hex.len() == 6
                    && u8::from_str_radix(&hex[0..2], 16).is_ok()
                    && u8::from_str_radix(&hex[2..4], 16).is_ok()
                    && u8::from_str_radix(&hex[4..6], 16).is_ok()
                {
                    app.config.themecolor = hex.to_string();
                    app.config.save()?;
                    app.set_status(format!("主题色已更新: #{}", hex));
                } else {
                    app.set_status("无效颜色值, 需要6位十六进制 (如 56B6C2)");
                    app.status_expiry = None;
                }
                app.theme_color_input.clear();
                app.theme_color_cursor = 0;
                app.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                if app.theme_color_cursor > 0 {
                    let mut chars: Vec<char> = app.theme_color_input.chars().collect();
                    chars.remove(app.theme_color_cursor - 1);
                    app.theme_color_input = chars.iter().collect();
                    app.theme_color_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if app.theme_color_cursor < app.theme_color_input.chars().count() {
                    let chars: Vec<char> = app.theme_color_input.chars().collect();
                    app.theme_color_input = chars[..app.theme_color_cursor].iter().collect::<String>() 
                        + &chars[app.theme_color_cursor + 1..].iter().collect::<String>();
                }
            }
            KeyCode::Left => {
                if app.theme_color_cursor > 0 {
                    app.theme_color_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if app.theme_color_cursor < app.theme_color_input.chars().count() {
                    app.theme_color_cursor += 1;
                }
            }
            KeyCode::Home => {
                app.theme_color_cursor = 0;
            }
            KeyCode::End => {
                app.theme_color_cursor = app.theme_color_input.chars().count();
            }
            KeyCode::Char(c) if c.is_ascii_hexdigit() || c == '#' => {
                let char_count = app.theme_color_input.chars().count();
                let max_len = if app.theme_color_input.starts_with('#') { 7 } else { 6 };
                if char_count < max_len {
                    let mut chars: Vec<char> = app.theme_color_input.chars().collect();
                    chars.insert(app.theme_color_cursor, c);
                    app.theme_color_input = chars.iter().collect();
                    app.theme_color_cursor += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    fn handle_switch_cache(&self, app: &mut App, audio_player: &mut AudioPlayer, lyrics_manager: &mut LyricsManager, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                if let Some(folder) = app.cached_folders.get(app.cached_folders_selected).cloned() {
                    app.switch_to_cached_folder(&folder, audio_player, lyrics_manager);
                }
                app.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !app.cached_folders.is_empty() {
                    app.cached_folders_selected = (app.cached_folders_selected + 1) % app.cached_folders.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !app.cached_folders.is_empty() {
                    app.cached_folders_selected = if app.cached_folders_selected == 0 {
                        app.cached_folders.len() - 1
                    } else {
                        app.cached_folders_selected - 1
                    };
                }
            }
            _ => {}
        }
        Ok(())
    }
}
