use std::{
    io::{Read, Seek, SeekFrom},
    sync::Arc,
};


/// Zero-copy in-memory reader for cached track bytes.
/// `Cursor<Arc<Vec<u8>>>` does not implement `Read/Seek`, so we wrap
/// `Arc<Vec<u8>> + position` manually. `Send + Sync + 'static` for rodio.
pub(crate) struct SharedAudio {
    data: Arc<Vec<u8>>,
    pos: u64,
}

impl SharedAudio {
    pub(crate) fn new(data: Arc<Vec<u8>>) -> Self {
        Self { data, pos: 0 }
    }
}

impl Read for SharedAudio {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = self.pos as usize;
        if start >= self.data.len() {
            return Ok(0);
        }
        let n = (self.data.len() - start).min(buf.len());
        buf[..n].copy_from_slice(&self.data[start..start + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for SharedAudio {
    fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
        let len = self.data.len() as i64;
        let new_pos: i64 = match style {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(n) => len + n,
            SeekFrom::Current(n) => self.pos as i64 + n,
        };
        let clamped = new_pos.max(0).min(len) as u64;
        self.pos = clamped;
        Ok(clamped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_audio_read_and_seek() {
        use std::io::{Read, Seek, SeekFrom};

        let data = Arc::new(vec![1u8, 2, 3, 4, 5]);
        let mut audio = SharedAudio::new(Arc::clone(&data));

        // Sequential reads advance position
        let mut buf = [0u8; 2];
        assert_eq!(audio.read(&mut buf).unwrap(), 2);
        assert_eq!(buf, [1, 2]);

        // Seek to absolute offset and read tail
        assert_eq!(audio.seek(SeekFrom::Start(4)).unwrap(), 4);
        let mut tail = [0u8; 2];
        assert_eq!(audio.read(&mut tail).unwrap(), 1);
        assert_eq!(tail[0], 5);

        // Seeking past end clamps, reads return EOF
        assert_eq!(audio.seek(SeekFrom::Start(99)).unwrap(), 5);
        assert_eq!(audio.read(&mut tail).unwrap(), 0);
    }
}
