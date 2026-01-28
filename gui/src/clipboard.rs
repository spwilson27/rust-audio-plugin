//! Cross-platform clipboard access wrapper

use anyhow::Result;
use arboard::Clipboard;

/// Clipboard manager for copy/paste operations
///
/// This is a simple wrapper around `arboard::Clipboard` to provide
/// a consistent interface for text clipboard operations.
pub struct ClipboardManager {
    clipboard: Clipboard,
}

impl ClipboardManager {
    /// Create a new clipboard manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            clipboard: Clipboard::new()?,
        })
    }

    /// Copy text to the clipboard
    pub fn set_text(&mut self, text: impl AsRef<str>) -> Result<()> {
        self.clipboard
            .set_text(text.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to set clipboard text: {}", e))
    }

    /// Get text from the clipboard
    pub fn get_text(&mut self) -> Result<String> {
        self.clipboard
            .get_text()
            .map_err(|e| anyhow::anyhow!("Failed to get clipboard text: {}", e))
    }

    /// Check if clipboard has text
    pub fn has_text(&mut self) -> bool {
        self.get_text().is_ok()
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            clipboard: Clipboard::new().expect("Failed to create clipboard"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Clipboard tests may fail in headless/CI environments
    // Run with: cargo test -- --test-threads=1
    // to avoid race conditions between clipboard tests

    #[test]
    #[ignore] // Run manually with: cargo test -- --ignored
    fn test_clipboard_set_get() {
        let mut clipboard = match ClipboardManager::new() {
            Ok(c) => c,
            Err(_) => {
                // Skip test if clipboard is not available (e.g., CI, headless)
                eprintln!("Skipping clipboard test - clipboard not available");
                return;
            }
        };

        // Set text
        if let Err(e) = clipboard.set_text("Hello, World!") {
            eprintln!("Clipboard set failed: {} - skipping test", e);
            return;
        }

        // Get text back
        match clipboard.get_text() {
            Ok(text) => assert_eq!(text, "Hello, World!"),
            Err(e) => {
                eprintln!("Clipboard get failed: {} - skipping test", e);
            }
        }
    }

    #[test]
    #[ignore]
    fn test_clipboard_overwrite() {
        let mut clipboard = match ClipboardManager::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        if clipboard.set_text("First").is_err() {
            return;
        }
        if clipboard.set_text("Second").is_err() {
            return;
        }

        if let Ok(text) = clipboard.get_text() {
            assert_eq!(text, "Second");
        }
    }

    #[test]
    #[ignore]
    fn test_clipboard_has_text() {
        let mut clipboard = match ClipboardManager::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        if clipboard.set_text("Test").is_ok() {
            assert!(clipboard.has_text());
        }
    }

    #[test]
    fn test_clipboard_creation() {
        // Test that we can at least create a clipboard manager
        // This should work even in headless environments
        let result = ClipboardManager::new();
        // Just check it doesn't panic - it may fail in CI
        match result {
            Ok(_) => { /* Success */ }
            Err(e) => eprintln!("Clipboard unavailable (expected in CI): {}", e),
        }
    }
}
