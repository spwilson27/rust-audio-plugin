pub const ROBOTO_REGULAR: &[u8] = include_bytes!("Roboto-Regular.ttf");

pub fn default_font() -> &'static [u8] {
    ROBOTO_REGULAR
}
