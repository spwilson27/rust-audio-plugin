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
    // macOS Keycodes (approximate, need to verify against Carbon/HIToolbox or empirical)
    // 0: A
    // 6: Z
    // 7: X
    // 8: C
    // 9: V
    // 51: Backspace

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
    // 51 is commonly Delete/Backspace
    if modifiers.contains(Modifiers::ALT) && keycode == 51 {
        return Some(StandardAction::DeleteWord);
    }

    // Option + Forward Delete (Delete Word Forward)
    // 117 is Forward Delete
    if modifiers.contains(Modifiers::ALT) && keycode == 117 {
        return Some(StandardAction::DeleteWordForward);
    }

    None
}

#[cfg(not(target_os = "macos"))]
pub fn match_shortcut(modifiers: Modifiers, keycode: u32) -> Option<StandardAction> {
    // Windows/Linux Keycodes (Standard Scancodes usually)
    // Assuming keycode comes from PAL which maps native.
    // If PAL passes raw native codes, this is hard.
    // Ideally PAL should normalize keycodes or passing char.
    // For now, let's assume raw scan codes similar to layout.

    // Ctrl + C
    if modifiers.contains(Modifiers::CTRL) && (keycode == 67 || keycode == 0x2E/* C */) {
        return Some(StandardAction::Copy);
    }
    // ... Stub for now as we are on Mac
    None
}
