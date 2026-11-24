//! Movable char-caret helpers shared by every browser text modal.

/// Byte index of a char-position caret (clamped to text end).
pub(super) fn caret_byte_idx(s: &str, caret: usize) -> usize {
    s.char_indices()
        .map(|(b, _)| b)
        .nth(caret)
        .unwrap_or(s.len())
}

/// Movable caret for single-line modal inputs (browser modals).
pub(super) fn caret_insert(s: &mut String, caret: &mut usize, c: char) {
    let byte = caret_byte_idx(s, *caret);
    s.insert(byte, c);
    *caret += 1;
}

pub(super) fn caret_backspace(s: &mut String, caret: &mut usize) {
    if *caret == 0 {
        return;
    }
    let end = caret_byte_idx(s, *caret);
    let start = caret_byte_idx(s, *caret - 1);
    s.drain(start..end);
    *caret -= 1;
}

pub(super) fn caret_delete(s: &mut String, caret: usize) {
    let len = s.chars().count();
    if caret >= len {
        return;
    }
    let start = caret_byte_idx(s, caret);
    let end = caret_byte_idx(s, caret + 1);
    s.drain(start..end);
}

pub(super) fn caret_delete_word(s: &mut String, caret: &mut usize) {
    let chars: Vec<char> = s.chars().collect();
    let mut start = (*caret).min(chars.len());
    while start > 0 && chars[start - 1] == ' ' {
        start -= 1;
    }
    while start > 0 && chars[start - 1] != ' ' {
        start -= 1;
    }
    let b_start = caret_byte_idx(s, start);
    let b_end = caret_byte_idx(s, (*caret).min(chars.len()));
    s.drain(b_start..b_end);
    *caret = start;
}

pub(super) fn caret_end(s: &str) -> usize {
    s.chars().count()
}

/// Mutable access to one EditTrack text field by index
/// (0: Title, 1: Artist, 2: Album, 3: File name).
pub(super) fn edit_field_mut<'a>(
    idx: usize,
    title: &'a mut String,
    artist: &'a mut String,
    album: &'a mut String,
    file_name: &'a mut String,
) -> Option<&'a mut String> {
    match idx {
        0 => Some(title),
        1 => Some(artist),
        2 => Some(album),
        3 => Some(file_name),
        _ => None,
    }
}

pub(super) fn edit_field(
    field_idx: usize,
    title: &str,
    artist: &str,
    album: &str,
    file_name: &str,
) -> usize {
    let text = match field_idx {
        0 => title,
        1 => artist,
        2 => album,
        3 => file_name,
        _ => "",
    };
    caret_end(text)
}
