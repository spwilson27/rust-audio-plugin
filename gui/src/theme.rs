//! Visual theme definitions
//!
//! Provides a centralized color palette and style constants for the UI.

pub type Color = [f32; 4];

pub mod colors {
    use super::Color;

    // Backgrounds
    pub const BACKGROUND_ROOT: Color = [0.05, 0.05, 0.05, 1.0]; // Deep black/gray
    pub const BACKGROUND_WIDGET: Color = [0.15, 0.15, 0.16, 1.0]; // Surface gray
    pub const BACKGROUND_HOVER: Color = [0.20, 0.20, 0.22, 1.0]; // Lighter gray
    pub const BACKGROUND_PRESSED: Color = [0.10, 0.10, 0.11, 1.0]; // Darker gray

    // Accents
    pub const ACCENT: Color = [0.0, 0.47, 0.95, 1.0]; // SF Blue-ish
    pub const ACCENT_DIM: Color = [0.0, 0.3, 0.6, 1.0];
    pub const ACCENT_HOVER: Color = [0.2, 0.6, 1.0, 1.0];

    // Text
    pub const TEXT_PRIMARY: Color = [0.92, 0.92, 0.92, 1.0];
    pub const TEXT_SECONDARY: Color = [0.60, 0.60, 0.60, 1.0];
    pub const TEXT_DISABLED: Color = [0.40, 0.40, 0.40, 1.0];

    // Borders
    pub const BORDER: Color = [0.25, 0.25, 0.25, 1.0];
    pub const BORDER_FOCUS: Color = [0.0, 0.47, 0.95, 1.0];
    pub const BORDER_HOVER: Color = [0.35, 0.35, 0.35, 1.0]; // Slight highlight

    // Feedback
    pub const SELECTION: Color = [0.1, 0.3, 0.6, 0.5];
    pub const CURSOR: Color = [0.9, 0.9, 0.9, 0.9];
}
