use pal::Modifiers;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardAction {
    Cut,
    Copy,
    Paste,
    SelectAll,
    DeleteWord,
    DeleteWordForward,
}

#[cfg(target_os = "macos")]
pub fn match_shortcut(modifiers: Modifiers, keycode: u32) -> Option<StandardAction> {
    // macOS Keycodes
    // 0: A
    // 7: X
    // 8: C
    // 9: V
    // 51: Backspace
    // 117: Delete

    // Command + C
    if modifiers.contains(Modifiers::META) && keycode == 8 {
        return Some(StandardAction::Copy);
    }
    // Command + V
    if modifiers.contains(Modifiers::META) && keycode == 9 {
        return Some(StandardAction::Paste);
    }
    // Command + X
    if modifiers.contains(Modifiers::META) && keycode == 7 {
        return Some(StandardAction::Cut);
    }
    // Command + A
    if modifiers.contains(Modifiers::META) && keycode == 0 {
        return Some(StandardAction::SelectAll);
    }

    // Option + Backspace (Delete Word)
    if modifiers.contains(Modifiers::ALT) && keycode == 51 {
        return Some(StandardAction::DeleteWord);
    }

    // Option + Forward Delete (Delete Word Forward)
    if modifiers.contains(Modifiers::ALT) && keycode == 117 {
        return Some(StandardAction::DeleteWordForward);
    }

    None
}

#[cfg(not(target_os = "macos"))]
pub fn match_shortcut(modifiers: Modifiers, keycode: u32) -> Option<StandardAction> {
    // Windows/Linux Shortcuts (Ctrl-based)
    // Keycodes vary by platform, but for E2E tests we may see macOS codes injected.

    // Copy: Ctrl+C (67) or Mac-injected Cmd+C (8)
    if (modifiers.contains(Modifiers::CTRL) && keycode == 67)
        || (modifiers.contains(Modifiers::META) && keycode == 8)
    {
        return Some(StandardAction::Copy);
    }

    // Paste: Ctrl+V (86) or Mac-injected Cmd+V (9)
    if (modifiers.contains(Modifiers::CTRL) && keycode == 86)
        || (modifiers.contains(Modifiers::META) && keycode == 9)
    {
        return Some(StandardAction::Paste);
    }

    // Cut: Ctrl+X (88) or Mac-injected Cmd+X (7)
    if (modifiers.contains(Modifiers::CTRL) && keycode == 88)
        || (modifiers.contains(Modifiers::META) && keycode == 7)
    {
        return Some(StandardAction::Cut);
    }

    // Select All: Ctrl+A (65) or Mac-injected Cmd+A (0)
    if (modifiers.contains(Modifiers::CTRL) && keycode == 65)
        || (modifiers.contains(Modifiers::META) && keycode == 0)
    {
        return Some(StandardAction::SelectAll);
    }

    // Delete Word: Ctrl+Backspace (8) or Mac-injected Option+Backspace (Alt+51)
    if (modifiers.contains(Modifiers::CTRL) && (keycode == 8 || keycode == 22))
        || (modifiers.contains(Modifiers::ALT) && keycode == 51)
    {
        return Some(StandardAction::DeleteWord);
    }

    // Delete Word Forward: Ctrl+Delete (46) or Mac-injected Option+Delete (Alt+117)
    if (modifiers.contains(Modifiers::CTRL) && (keycode == 46 || keycode == 127))
        || (modifiers.contains(Modifiers::ALT) && keycode == 117)
    {
        return Some(StandardAction::DeleteWordForward);
    }

    None
}
