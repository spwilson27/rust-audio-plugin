pub const ROBOTO_REGULAR: &[u8] = include_bytes!("Roboto-Regular.ttf");

pub fn default_font() -> &'static [u8] {
    ROBOTO_REGULAR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_integrity() {
        assert!(!ROBOTO_REGULAR.is_empty());
        assert_eq!(ROBOTO_REGULAR[0..4], [0x00, 0x01, 0x00, 0x00]); // TTF signature usually
    }
}
