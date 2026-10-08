use std::io::{Cursor, Write};
use std::path::Path;
use image::{imageops::FilterType, DynamicImage, ImageFormat, RgbaImage};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn is_kitty_supported() -> bool {
    std::env::var("KITTY_WINDOW_ID").is_ok()
        || std::env::var("KITTY_PID").is_ok()
        || std::env::var("GHOSTTY_RESOURCES_DIR").is_ok()
        || std::env::var("WEZTERM_PANE").is_ok()
        || std::env::var("TERM_PROGRAM")
            .map(|p| {
                let p = p.to_ascii_lowercase();
                p == "wezterm" || p == "ghostty" || p == "kitty"
            })
            .unwrap_or(false)
        || std::env::var("TERM")
            .map(|t| t.contains("kitty") || t.contains("ghostty") || t.contains("wezterm"))
            .unwrap_or(false)
}

fn to_base64(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_CHARS[(triple >> 18) & 0x3F] as char);
        out.push(B64_CHARS[(triple >> 12) & 0x3F] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[(triple >> 6) & 0x3F] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[triple & 0x3F] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub struct AlbumArt {
    pub image: Option<RgbaImage>,
    pub png_base64: Option<String>,
}

impl AlbumArt {
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            image: None,
            png_base64: None,
        }
    }

    pub fn from_dynamic_image(dyn_img: DynamicImage) -> Self {
        let rgba = dyn_img.to_rgba8();

        let mut png_base64 = None;
        if is_kitty_supported() {
            // Scale to a clean 400x400 thumbnail for sharp display with minimal bandwidth
            let thumb = dyn_img.resize_to_fill(400, 400, FilterType::Lanczos3);
            let mut png_bytes = Vec::new();
            if thumb.write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png).is_ok() {
                png_base64 = Some(to_base64(&png_bytes));
            }
        }

        Self {
            image: Some(rgba),
            png_base64,
        }
    }

    /// Loads album art from embedded ID3 APIC frame, or fallback image files in the same folder
    pub fn load_for_song(song_path: &Path) -> Self {        // 1. Try reading embedded ID3 APIC picture
        if let Ok(tag) = id3::Tag::read_from_path(song_path) {
            for picture in tag.pictures() {
                if let Ok(dyn_img) = image::load_from_memory(&picture.data) {
                    return Self::from_dynamic_image(dyn_img);
                }
            }
        }

        // 2. Check for cover files in the same directory
        if let Some(parent) = song_path.parent() {
            let names = ["cover.jpg", "cover.png", "folder.jpg", "album.jpg"];
            for name in names {
                let path = parent.join(name);
                if path.exists() {
                    if let Ok(dyn_img) = image::open(&path) {
                        return Self::from_dynamic_image(dyn_img);
                    }
                }
            }

            for ext in &["jpg", "jpeg", "png", "webp"] {
                let path = song_path.with_extension(ext);
                if path.exists() {
                    if let Ok(dyn_img) = image::open(&path) {
                        return Self::from_dynamic_image(dyn_img);
                    }
                }
            }
        }

        Self {
            image: None,
            png_base64: None,
        }
    }

    /// Saves the cover as a PNG file (used for the MPRIS `artUrl`).
    /// Returns `true` on success.
    #[cfg(feature = "mpris")]
    pub fn save_png(&self, path: &Path) -> bool {
        match &self.image {
            Some(img) => img.save(path).is_ok(),
            None => false,
        }
    }

    /// Renders the image into Ratatui Lines using Unicode halfblocks (▀)
    pub fn render_halfblocks(&self, width: u16, height: u16) -> Vec<Line<'static>> {
        self.render_halfblocks_dimmed(width, height, 1.0)
    }

    /// Same as [`Self::render_halfblocks`], but scales every channel by
    /// `dim` (0.0-1.0). Used to paint a dimmed full-panel backdrop behind
    /// the sharp centered square so wide panels don't look empty.
    pub fn render_halfblocks_dimmed(
        &self,
        width: u16,
        height: u16,
        dim: f32,
    ) -> Vec<Line<'static>> {
        let img = match &self.image {
            Some(i) => i,
            None => return Vec::new(),
        };

        if width == 0 || height == 0 {
            return Vec::new();
        }

        let target_w = width as u32;
        let target_h = (height * 2) as u32;

        let resized = image::imageops::resize(
            img,
            target_w,
            target_h,
            FilterType::Triangle,
        );

        let dim_ch = |v: u8| ((v as f32) * dim.clamp(0.0, 1.0)) as u8;

        let mut lines = Vec::with_capacity(height as usize);

        for y in 0..height as u32 {
            let top_y = y * 2;
            let bot_y = (y * 2 + 1).min(target_h - 1);

            let mut spans = Vec::with_capacity(width as usize);

            for x in 0..target_w {
                let top_px = resized.get_pixel(x, top_y);
                let bot_px = resized.get_pixel(x, bot_y);

                let fg = Color::Rgb(dim_ch(top_px[0]), dim_ch(top_px[1]), dim_ch(top_px[2]));
                let bg = Color::Rgb(dim_ch(bot_px[0]), dim_ch(bot_px[1]), dim_ch(bot_px[2]));

                spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
            }

            lines.push(Line::from(spans));
        }

        lines
    }
}

/// Transmits and displays an image using the Kitty Graphics Protocol with chunking
pub fn write_kitty_image<W: Write>(
    writer: &mut W,
    b64: &str,
    x: u16,
    y: u16,
    cols: u16,
    rows: u16,
) -> std::io::Result<()> {
    if b64.is_empty() || cols == 0 || rows == 0 {
        return Ok(());
    }

    // Position terminal cursor to the top-left cell of the destination rectangle (1-indexed)
    write!(writer, "\x1b[{};{}H", y + 1, x + 1)?;

    let chunk_size = 4096;
    let bytes = b64.as_bytes();
    let total_len = bytes.len();
    let mut offset = 0;
    let mut is_first = true;

    while offset < total_len {
        let end = (offset + chunk_size).min(total_len);
        let chunk_str = &b64[offset..end];
        let has_more = end < total_len;
        let m = if has_more { 1 } else { 0 };

        if is_first {
            // First chunk carries all metadata: action T (display), format 100 (png), id 1, cols, rows, quiet 2, no cursor advance C=1
            write!(
                writer,
                "\x1b_Ga=T,f=100,i=1,c={},r={},q=2,C=1,m={};{}\x1b\\",
                cols, rows, m, chunk_str
            )?;
            is_first = false;
        } else {
            write!(writer, "\x1b_Gm={};{}\x1b\\", m, chunk_str)?;
        }

        offset = end;
    }

    writer.flush()?;
    Ok(())
}

/// Clears any Kitty Graphics Protocol images displayed in the terminal
pub fn clear_kitty_image<W: Write>(writer: &mut W) -> std::io::Result<()> {
    write!(writer, "\x1b_Ga=d,d=a,q=2\x1b\\")?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_standard_vectors() {
        assert_eq!(to_base64(b""), "");
        assert_eq!(to_base64(b"f"), "Zg==");
        assert_eq!(to_base64(b"fo"), "Zm8=");
        assert_eq!(to_base64(b"foo"), "Zm9v");
        assert_eq!(to_base64(b"foob"), "Zm9vYg==");
        assert_eq!(to_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(to_base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(to_base64(b"hello world"), "aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn test_load_album_art_from_file() {
        let fake_path = Path::new("non_existent_cover_test.mp3");
        let art = AlbumArt::load_for_song(fake_path);
        assert!(art.image.is_none());
        assert!(art.png_base64.is_none());
    }
}
