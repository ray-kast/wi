use keyboard_types::{Modifiers, NamedKey};
use ratatui::crossterm::event::{
    KeyCode, KeyEventState, KeyModifiers, MediaKeyCode, ModifierKeyCode,
};

use crate::{
    graph::TuiNode,
    widget::{Cx, GraphEditor},
};

impl<N: TuiNode> GraphEditor<N> {
    #[expect(clippy::too_many_lines, reason = "Oops! All match block")]
    pub fn handle_crossterm_key(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
        state: KeyEventState,
        cx: &mut Cx<'_>,
    ) -> bool {
        let _ = state;
        let mut mods_out = Modifiers::empty();
        for modifier in mods.iter() {
            mods_out.insert(match modifier {
                KeyModifiers::ALT => Modifiers::ALT,
                KeyModifiers::META | KeyModifiers::SUPER | KeyModifiers::HYPER => Modifiers::META,
                KeyModifiers::SHIFT => Modifiers::SHIFT,
                KeyModifiers::CONTROL => Modifiers::CONTROL,
                m => panic!("Unknown modifier(s): {m}"),
            });
        }

        let key = match code {
            KeyCode::Backspace => NamedKey::Backspace,
            KeyCode::Enter => NamedKey::Enter,
            KeyCode::Left => NamedKey::ArrowLeft,
            KeyCode::Right => NamedKey::ArrowRight,
            KeyCode::Up => NamedKey::ArrowUp,
            KeyCode::Down => NamedKey::ArrowDown,
            KeyCode::Home => NamedKey::Home,
            KeyCode::End => NamedKey::End,
            KeyCode::PageUp => NamedKey::PageUp,
            KeyCode::PageDown => NamedKey::PageDown,
            KeyCode::Tab => NamedKey::Tab,
            KeyCode::Delete => NamedKey::Delete,
            KeyCode::Insert => NamedKey::Insert,
            KeyCode::F(1) => NamedKey::F1,
            KeyCode::F(2) => NamedKey::F2,
            KeyCode::F(3) => NamedKey::F3,
            KeyCode::F(4) => NamedKey::F4,
            KeyCode::F(5) => NamedKey::F5,
            KeyCode::F(6) => NamedKey::F6,
            KeyCode::F(7) => NamedKey::F7,
            KeyCode::F(8) => NamedKey::F8,
            KeyCode::F(9) => NamedKey::F9,
            KeyCode::F(10) => NamedKey::F10,
            KeyCode::F(11) => NamedKey::F11,
            KeyCode::F(12) => NamedKey::F12,
            KeyCode::F(13) => NamedKey::F13,
            KeyCode::F(14) => NamedKey::F14,
            KeyCode::F(15) => NamedKey::F15,
            KeyCode::F(16) => NamedKey::F16,
            KeyCode::F(17) => NamedKey::F17,
            KeyCode::F(18) => NamedKey::F18,
            KeyCode::F(19) => NamedKey::F19,
            KeyCode::F(20) => NamedKey::F20,
            KeyCode::F(21) => NamedKey::F21,
            KeyCode::F(22) => NamedKey::F22,
            KeyCode::F(23) => NamedKey::F23,
            KeyCode::F(24) => NamedKey::F24,
            KeyCode::F(25) => NamedKey::F25,
            KeyCode::F(26) => NamedKey::F26,
            KeyCode::F(27) => NamedKey::F27,
            KeyCode::F(28) => NamedKey::F28,
            KeyCode::F(29) => NamedKey::F29,
            KeyCode::F(30) => NamedKey::F30,
            KeyCode::F(31) => NamedKey::F31,
            KeyCode::F(32) => NamedKey::F32,
            KeyCode::F(33) => NamedKey::F33,
            KeyCode::F(34) => NamedKey::F34,
            KeyCode::F(35) => NamedKey::F35,
            KeyCode::Char(c) => return self.handle_char_input(&c.to_string(), mods_out, cx),
            KeyCode::Esc => NamedKey::Escape,
            KeyCode::CapsLock => NamedKey::CapsLock,
            KeyCode::ScrollLock => NamedKey::ScrollLock,
            KeyCode::NumLock => NamedKey::NumLock,
            KeyCode::PrintScreen => NamedKey::PrintScreen,
            KeyCode::Pause => NamedKey::Pause,
            KeyCode::Menu => NamedKey::ContextMenu,
            KeyCode::Media(MediaKeyCode::Play) => NamedKey::MediaPlay,
            KeyCode::Media(MediaKeyCode::Pause) => NamedKey::MediaPause,
            KeyCode::Media(MediaKeyCode::PlayPause) => NamedKey::MediaPlayPause,
            KeyCode::Media(MediaKeyCode::Stop) => NamedKey::MediaStop,
            KeyCode::Media(MediaKeyCode::FastForward) => NamedKey::MediaFastForward,
            KeyCode::Media(MediaKeyCode::Rewind) => NamedKey::MediaRewind,
            KeyCode::Media(MediaKeyCode::TrackNext) => NamedKey::MediaTrackNext,
            KeyCode::Media(MediaKeyCode::TrackPrevious) => NamedKey::MediaTrackPrevious,
            KeyCode::Media(MediaKeyCode::Record) => NamedKey::MediaRecord,
            KeyCode::Media(MediaKeyCode::LowerVolume) => NamedKey::AudioVolumeDown,
            KeyCode::Media(MediaKeyCode::RaiseVolume) => NamedKey::AudioVolumeUp,
            KeyCode::Media(MediaKeyCode::MuteVolume) => NamedKey::AudioVolumeMute,
            KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) => {
                NamedKey::Shift
            },
            KeyCode::Modifier(ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl) => {
                NamedKey::Control
            },
            KeyCode::Modifier(ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt) => {
                NamedKey::Alt
            },
            KeyCode::Modifier(
                ModifierKeyCode::LeftSuper
                | ModifierKeyCode::LeftHyper
                | ModifierKeyCode::LeftMeta
                | ModifierKeyCode::RightSuper
                | ModifierKeyCode::RightHyper
                | ModifierKeyCode::RightMeta,
            ) => NamedKey::Meta,
            KeyCode::BackTab
            | KeyCode::F(0 | 36..)
            | KeyCode::Null
            | KeyCode::KeypadBegin
            | KeyCode::Media(MediaKeyCode::Reverse)
            | KeyCode::Modifier(
                ModifierKeyCode::IsoLevel3Shift | ModifierKeyCode::IsoLevel5Shift,
            ) => NamedKey::Unidentified,
        };

        self.handle_named_keypress(key, mods_out, cx)
    }
}
