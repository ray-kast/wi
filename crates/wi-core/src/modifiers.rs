use keyboard_types::Modifiers;

pub const M_NONE: Modifiers = Modifiers::empty();

#[cfg(not(target_vendor = "apple"))]
/// Default CTRL key, rebound to CMD for Apple targets
pub const M_CTRL: Modifiers = Modifiers::CONTROL;
#[cfg(target_vendor = "apple")]
/// Default CTRL key, rebound to CMD for Apple targets
pub const M_CTRL: Modifiers = Modifiers::META;

/// "Terminal" CTRL, i.e. not CMD on Apple targets
pub const M_TCTL: Modifiers = Modifiers::CONTROL;

#[cfg(not(target_vendor = "apple"))]
/// Default CMD key, rebound to CTRL+SHIFT on non-Apple targets
pub const M_CMD: Modifiers = Modifiers::CONTROL.union(Modifiers::SHIFT);
#[cfg(target_vendor = "apple")]
/// Default CMD key, rebound to CTRL+SHIFT on non-Apple targets
pub const M_CMD: Modifiers = Modifiers::META;

pub const M_ALT: Modifiers = Modifiers::ALT;
pub const M_SHIFT: Modifiers = Modifiers::SHIFT;
