use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static DECODER_PANIC: AtomicBool = AtomicBool::new(false);

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    _stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    start_time: Option<Instant>,
    paused_at: Option<Duration>,
    is_paused: bool,
    current_path: Option<String>,
    total_duration: Option<Duration>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            _stream: None,
            _stream_handle: None,
            sink: None,
            start_time: None,
            paused_at: None,
            is_paused: false,
            current_path: None,
            total_duration: None,
        })
    }
    
    fn create_decoder(path: &str) -> Result<Decoder<BufReader<File>>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let path_owned = path.to_string();
        
        // Use spawn + catch_unwind to isolate potential panics from symphonia decoder
        DECODER_PANIC.store(false, Ordering::SeqCst);
        
        let handle = thread::spawn(move || {
            // Set a panic hook that just sets our flag
            let old_hook = panic::take_hook();
            panic::set_hook(Box::new(|_| {
                DECODER_PANIC.store(true, Ordering::SeqCst);
            }));
            
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                Decoder::new(reader)
            }));
            
            // Restore old hook
            panic::set_hook(old_hook);
            
            result
        });
        
        match handle.join() {
            Ok(Ok(Ok(decoder))) => Ok(decoder),
            Ok(Ok(Err(e))) => Err(e.into()),
            Ok(Err(_)) | Err(_) => anyhow::bail!("无法解码音频文件 '{}' - 格式不兼容或文件损坏", path_owned),
        }
    }
    
    pub fn play(&mut self, path: &str) -> Result<()> {
        // Stop current playback
        self.stop();
        
        // Create new audio stream
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        
        let source = Self::create_decoder(path)?;
        
        // Try to get total duration
        self.total_duration = source.total_duration();
        
        sink.append(source);
        sink.pause(); // Pause immediately, then resume to sync timing
        sink.play();
        
        self._stream = Some(stream);
        self._stream_handle = Some(stream_handle);
        self.sink = Some(sink);
        self.start_time = Some(Instant::now());
        self.paused_at = None;
        self.is_paused = false;
        self.current_path = Some(path.to_string());
        
        Ok(())
    }
    
    pub fn toggle_pause(&mut self) {
        if let Some(ref sink) = self.sink {
            if self.is_paused {
                sink.play();
                self.is_paused = false;
                // Adjust start time to account for pause duration
                if let Some(paused) = self.paused_at {
                    if let Some(start) = self.start_time.as_mut() {
                        *start = Instant::now() - paused;
                    }
                }
            } else {
                self.paused_at = Some(self.current_position());
                sink.pause();
                self.is_paused = true;
            }
        }
    }
    
    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self._stream = None;
        self._stream_handle = None;
        self.start_time = None;
        self.paused_at = None;
        self.is_paused = false;
    }
    
    pub fn current_position(&self) -> Duration {
        if let Some(start) = self.start_time {
            if self.is_paused {
                self.paused_at.unwrap_or(Duration::ZERO)
            } else {
                Instant::now().duration_since(start)
            }
        } else {
            Duration::ZERO
        }
    }
    
    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().map_or(false, |s| !s.is_paused() && !s.empty())
    }
    
    pub fn set_volume(&mut self, volume: f32) {
        if let Some(ref sink) = self.sink {
            sink.set_volume(volume);
        }
    }
    
    pub fn get_volume(&self) -> f32 {
        self.sink.as_ref().map_or(1.0, |s| s.volume())
    }
    
    pub fn seek_relative(&mut self, forward: bool, secs: u64) -> Result<()> {
        let path = match &self.current_path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        
        // Calculate new position
        let current = self.current_position();
        let offset = Duration::from_secs(secs);
        let new_pos = if forward {
            current.saturating_add(offset)
        } else {
            current.saturating_sub(offset)
        };
        
        // Clamp to valid range
        let new_pos = if let Some(total) = self.total_duration {
            new_pos.min(total)
        } else {
            new_pos
        };
        
        // Don't seek to very beginning
        let new_pos = new_pos.max(Duration::ZERO);
        
        // Stop and restart with skip
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self._stream = None;
        self._stream_handle = None;
        
        // Create new audio stream
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        
        let source = Self::create_decoder(&path)?
            .skip_duration(new_pos);
        
        sink.append(source);
        if self.is_paused {
            sink.pause();
        } else {
            sink.play();
        }
        
        self._stream = Some(stream);
        self._stream_handle = Some(stream_handle);
        self.sink = Some(sink);
        self.start_time = Some(Instant::now() - new_pos);
        self.paused_at = if self.is_paused { Some(new_pos) } else { None };
        
        Ok(())
    }
}
