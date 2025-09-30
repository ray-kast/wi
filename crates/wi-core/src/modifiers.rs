use keyboard_types::Modifiers;

pub const M_NONE: Modifiers = Modifiers::empty();

#[cfg(not(target_vendor = "apple"))]
pub const M_CTRL: Modifiers = Modifiers::CONTROL;
#[cfg(target_vendor = "apple")]
pub const M_CTRL: Modifiers = Modifiers::META;

pub const M_SHIFT: Modifiers = Modifiers::SHIFT;
