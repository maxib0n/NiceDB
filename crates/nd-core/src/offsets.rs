pub(crate) fn utf16_to_byte(text: &str, utf16: u32) -> usize {
    let mut units = 0u32;
    for (byte, ch) in text.char_indices() {
        if units >= utf16 {
            return byte;
        }
        units += u32::try_from(ch.len_utf16()).unwrap_or(2);
    }
    text.len()
}

pub(crate) fn byte_to_utf16(text: &str, byte: usize) -> u32 {
    let end = text.floor_char_boundary(byte.min(text.len()));
    u32::try_from(text[..end].encode_utf16().count()).unwrap_or(u32::MAX)
}

pub(crate) struct Utf16Cursor<'a> {
    text: &'a str,
    byte: usize,
    units: u32,
}

impl<'a> Utf16Cursor<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            text,
            byte: 0,
            units: 0,
        }
    }

    pub(crate) fn at(&mut self, byte: usize) -> u32 {
        let end = self.text.floor_char_boundary(byte.min(self.text.len()));
        if end < self.byte {
            (self.byte, self.units) = (0, 0);
        }
        let more = self.text[self.byte..end].encode_utf16().count();
        self.units = self
            .units
            .saturating_add(u32::try_from(more).unwrap_or(u32::MAX));
        self.byte = end;
        self.units
    }
}
