use std::{fs::File, io::BufReader, path::Path};
use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink};

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
    pub is_paused: bool,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        Ok(Self {
            _stream,
            sink,
            is_paused: false,
        })
    }

    pub fn play(&mut self, song_path: &Path) -> Result<()> {
        let file = File::open(song_path)?;
        let buffered_reader = BufReader::new(file);
        let audio_source = Decoder::new(buffered_reader)?;

        self.sink.clear();
        self.sink.append(audio_source);
        self.sink.play();
        self.is_paused = false;

        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            self.sink.play();
            self.is_paused = false;
        } else {
            self.sink.pause();
            self.is_paused = true;
        }
    }
}
