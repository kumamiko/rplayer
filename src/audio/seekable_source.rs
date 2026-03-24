use rodio::Source;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;
use std::fs::File;
use std::time::Duration;

/// A rodio Source backed by symphonia with native seeking support.
pub struct SeekableSource {
    format_reader: Box<dyn FormatReader>,
    track_id: u32,
    symphonia_decoder: Box<dyn symphonia::core::codecs::Decoder>,
    sample_buffer: Option<SampleBuffer<f32>>,
    sample_buffer_pos: usize,
    sample_rate: u32,
    channels: u16,
    total_duration: Option<Duration>,
    current_duration: Duration,
}

impl SeekableSource {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let meta_opts = MetadataOptions::default();

        let probed = get_probe()
            .format(&hint, mss, &format_opts, &meta_opts)
            .map_err(|e| anyhow::anyhow!("不支持的格式 '{}': {}", path, e))?;

        let format_reader = probed.format;

        let track = format_reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow::anyhow!("文件 '{}' 中没有找到音频轨道", path))?;

        let track_id = track.id;

        let decoder_opts = DecoderOptions {
            verify: false,
            ..Default::default()
        };

        let symphonia_decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .map_err(|e| anyhow::anyhow!("无法创建解码器 '{}': {}", path, e))?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        let total_duration = track
            .codec_params
            .time_base
            .zip(track.codec_params.n_frames)
            .map(|(tb, frames)| tb.calc_time(frames).into());

        Ok(Self {
            format_reader,
            track_id,
            symphonia_decoder,
            sample_buffer: None,
            sample_buffer_pos: 0,
            sample_rate,
            channels,
            total_duration,
            current_duration: Duration::ZERO,
        })
    }

    #[allow(dead_code)]
    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    /// Seek to a specific timestamp using symphonia's native seeking (O(1) via seek table).
    pub fn seek_to(&mut self, pos: Duration) -> anyhow::Result<()> {
        let seek_to = SeekTo::Time {
            time: pos.into(),
            track_id: Some(self.track_id),
        };

        self.sample_buffer = None;
        self.sample_buffer_pos = 0;

        self.format_reader
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| anyhow::anyhow!("Seek 失败: {}", e))?;

        let _ = self.symphonia_decoder.reset();

        self.current_duration = pos;
        Ok(())
    }

    fn decode_next_packet(&mut self) -> bool {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    let _ = self.symphonia_decoder.reset();
                    continue;
                }
                Err(_) => return false,
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let decoded = match self.symphonia_decoder.decode(&packet) {
                Ok(d) => d,
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    let _ = self.symphonia_decoder.reset();
                    continue;
                }
                Err(_) => return false,
            };

            let spec = *decoded.spec();
            let duration = decoded.capacity() as u64;
            let mut buf = SampleBuffer::<f32>::new(duration, spec);
            buf.copy_interleaved_ref(decoded);
            self.sample_buffer = Some(buf);
            self.sample_buffer_pos = 0;
            return true;
        }
    }
}

impl Iterator for SeekableSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let buf_ready = self
            .sample_buffer
            .as_ref()
            .map_or(false, |b| self.sample_buffer_pos < b.len());

        if !buf_ready {
            if !self.decode_next_packet() {
                return None;
            }
        }

        let buf = self.sample_buffer.as_ref()?;
        if self.sample_buffer_pos < buf.len() {
            let sample = buf.samples()[self.sample_buffer_pos];
            self.sample_buffer_pos += 1;
            self.current_duration += Duration::from_secs_f64(1.0 / self.sample_rate as f64);
            return Some(sample);
        }

        None
    }
}

impl Source for SeekableSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }
}
