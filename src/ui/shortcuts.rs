//! Keyboard shortcut routing for transport (play/pause, prev/next, stop) and dialog dismiss.
//!
//! Pure functions of key + UI context so tests do not need to spin egui or an audio device.

use egui::{Key, Modifiers};

/// Which player receives transport shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePlayer {
    /// Bottom main player (playlist).
    Main,
    /// Add-audio or replace-audio dialog preview.
    Preview,
}

/// Logical shortcut key after mapping from egui input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    Space,
    Escape,
    PreviousTrack,
    NextTrack,
    Stop,
}

/// UI context that affects whether a key is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutContext {
    /// True when a text field (not a button/slider) owns keyboard focus.
    pub text_input_focused: bool,
    pub active_player: ActivePlayer,
    /// True when the add-audio or replace-audio dialog is open.
    pub dialog_open: bool,
}

/// Action the UI should take. The caller applies it to the active player / dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    TogglePlayPause,
    PreviousTrack,
    NextTrack,
    Stop,
    DismissDialog,
}

/// Map an egui key + modifiers to a logical shortcut key.
///
/// Bindings:
/// - Space: play/pause
/// - Escape: dismiss dialog
/// - `[` or Ctrl/Cmd+Left: previous track
/// - `]` or Ctrl/Cmd+Right: next track
/// - `S`: stop
#[must_use]
pub fn shortcut_key_from_egui(key: Key, modifiers: Modifiers) -> Option<ShortcutKey> {
    if modifiers.alt {
        return None;
    }
    let command = modifiers.command;
    let shift = modifiers.shift;
    match key {
        Key::Space if !command && !shift => Some(ShortcutKey::Space),
        Key::Escape if !command && !shift => Some(ShortcutKey::Escape),
        Key::S if !command && !shift => Some(ShortcutKey::Stop),
        Key::OpenBracket if !command && !shift => Some(ShortcutKey::PreviousTrack),
        Key::CloseBracket if !command && !shift => Some(ShortcutKey::NextTrack),
        Key::ArrowLeft if command && !shift => Some(ShortcutKey::PreviousTrack),
        Key::ArrowRight if command && !shift => Some(ShortcutKey::NextTrack),
        _ => None,
    }
}

const SHORTCUT_CANDIDATE_KEYS: &[Key] = &[
    Key::Escape,
    Key::Space,
    Key::S,
    Key::OpenBracket,
    Key::CloseBracket,
    Key::ArrowLeft,
    Key::ArrowRight,
];

/// First matching shortcut key pressed this frame, plus the egui key to consume.
#[must_use]
pub fn detect_pressed_shortcut(input: &egui::InputState) -> Option<(ShortcutKey, Key)> {
    for &key in SHORTCUT_CANDIDATE_KEYS {
        if input.key_pressed(key)
            && let Some(shortcut) = shortcut_key_from_egui(key, input.modifiers)
        {
            return Some((shortcut, key));
        }
    }
    None
}

/// Modifiers to pass to [`egui::InputState::consume_key`] for a detected egui key.
#[must_use]
pub fn consume_modifiers_for_key(key: Key) -> Modifiers {
    match key {
        Key::ArrowLeft | Key::ArrowRight => Modifiers::COMMAND,
        _ => Modifiers::NONE,
    }
}

/// Route a logical shortcut. Returns `None` when the key must be left for the widget
/// (typing in a text field, or a transport key that does not apply).
#[must_use]
pub fn route_shortcut(key: ShortcutKey, context: ShortcutContext) -> Option<ShortcutAction> {
    // Escape still dismisses an open dialog while a name/id field is focused.
    if key == ShortcutKey::Escape {
        if context.dialog_open {
            return Some(ShortcutAction::DismissDialog);
        }
        return None;
    }

    if context.text_input_focused {
        return None;
    }

    match key {
        ShortcutKey::Space => Some(ShortcutAction::TogglePlayPause),
        ShortcutKey::Stop => Some(ShortcutAction::Stop),
        ShortcutKey::PreviousTrack => match context.active_player {
            ActivePlayer::Main => Some(ShortcutAction::PreviousTrack),
            ActivePlayer::Preview => None,
        },
        ShortcutKey::NextTrack => match context.active_player {
            ActivePlayer::Main => Some(ShortcutAction::NextTrack),
            ActivePlayer::Preview => None,
        },
        ShortcutKey::Escape => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActivePlayer, ShortcutAction, ShortcutContext, ShortcutKey, route_shortcut,
        shortcut_key_from_egui,
    };
    use egui::{Key, Modifiers};

    fn ctx(text: bool, player: ActivePlayer, dialog: bool) -> ShortcutContext {
        ShortcutContext {
            text_input_focused: text,
            active_player: player,
            dialog_open: dialog,
        }
    }

    #[test]
    fn transport_route_space_toggles_play_pause_main() {
        let action = route_shortcut(ShortcutKey::Space, ctx(false, ActivePlayer::Main, false));
        assert_eq!(
            action,
            Some(ShortcutAction::TogglePlayPause),
            "Space on the main player must toggle play/pause"
        );
    }

    #[test]
    fn transport_route_space_toggles_play_pause_preview() {
        let action = route_shortcut(ShortcutKey::Space, ctx(false, ActivePlayer::Preview, true));
        assert_eq!(
            action,
            Some(ShortcutAction::TogglePlayPause),
            "Space with an add/replace preview open must target that preview (TogglePlayPause), not prev/next"
        );
    }

    #[test]
    fn transport_route_space_ignored_when_text_focused() {
        let action = route_shortcut(ShortcutKey::Space, ctx(true, ActivePlayer::Main, false));
        assert_eq!(
            action, None,
            "Space must not steal from a focused text field"
        );
        let preview = route_shortcut(ShortcutKey::Space, ctx(true, ActivePlayer::Preview, true));
        assert_eq!(
            preview, None,
            "Space must not steal from name/id fields in the add/replace dialog"
        );
    }

    #[test]
    fn transport_route_prev_next_stop_on_main() {
        let main = ctx(false, ActivePlayer::Main, false);
        assert_eq!(
            route_shortcut(ShortcutKey::PreviousTrack, main),
            Some(ShortcutAction::PreviousTrack),
            "previous-track key must map when the main player is active"
        );
        assert_eq!(
            route_shortcut(ShortcutKey::NextTrack, main),
            Some(ShortcutAction::NextTrack),
            "next-track key must map when the main player is active"
        );
        assert_eq!(
            route_shortcut(ShortcutKey::Stop, main),
            Some(ShortcutAction::Stop),
            "stop key must map when the main player is active"
        );
    }

    #[test]
    fn transport_route_prev_next_not_on_preview() {
        let preview = ctx(false, ActivePlayer::Preview, true);
        assert_eq!(
            route_shortcut(ShortcutKey::PreviousTrack, preview),
            None,
            "previous-track must not control the main playlist while a preview dialog is open"
        );
        assert_eq!(
            route_shortcut(ShortcutKey::NextTrack, preview),
            None,
            "next-track must not control the main playlist while a preview dialog is open"
        );
        assert_eq!(
            route_shortcut(ShortcutKey::Stop, preview),
            Some(ShortcutAction::Stop),
            "stop still applies to the active preview player"
        );
    }

    #[test]
    fn transport_route_escape_dismisses_dialog() {
        let open = ctx(false, ActivePlayer::Preview, true);
        assert_eq!(
            route_shortcut(ShortcutKey::Escape, open),
            Some(ShortcutAction::DismissDialog),
            "Escape must dismiss an open add/replace dialog without confirming"
        );
        let typing = ctx(true, ActivePlayer::Preview, true);
        assert_eq!(
            route_shortcut(ShortcutKey::Escape, typing),
            Some(ShortcutAction::DismissDialog),
            "Escape must still dismiss the dialog while a text field is focused"
        );
    }

    #[test]
    fn transport_route_escape_ignored_without_dialog() {
        let none = route_shortcut(ShortcutKey::Escape, ctx(false, ActivePlayer::Main, false));
        assert_eq!(
            none, None,
            "Escape must not fire dismiss when no add/replace dialog is open"
        );
    }

    #[test]
    fn transport_shortcut_key_from_egui() {
        assert_eq!(
            shortcut_key_from_egui(Key::Space, Modifiers::NONE),
            Some(ShortcutKey::Space),
            "unmodified Space is play/pause"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::Escape, Modifiers::NONE),
            Some(ShortcutKey::Escape),
            "unmodified Escape is dismiss"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::S, Modifiers::NONE),
            Some(ShortcutKey::Stop),
            "unmodified S is stop"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::OpenBracket, Modifiers::NONE),
            Some(ShortcutKey::PreviousTrack),
            "[ is previous track"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::CloseBracket, Modifiers::NONE),
            Some(ShortcutKey::NextTrack),
            "] is next track"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::ArrowLeft, Modifiers::COMMAND),
            Some(ShortcutKey::PreviousTrack),
            "Ctrl/Cmd+Left is previous track"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::ArrowRight, Modifiers::COMMAND),
            Some(ShortcutKey::NextTrack),
            "Ctrl/Cmd+Right is next track"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::Space, Modifiers::COMMAND),
            None,
            "modified Space is not play/pause"
        );
        assert_eq!(
            shortcut_key_from_egui(Key::ArrowLeft, Modifiers::NONE),
            None,
            "unmodified arrows are left for sliders"
        );
    }
}
