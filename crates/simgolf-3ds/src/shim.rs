//! The input types the `simgolf` crate takes from miniquad, with the same names and variants, so its UI modules compile
//! against this frontend unchanged once their `use miniquad::...` lines point here (`use crate::shim::*`). Mapping from the
//! 3DS's buttons, circle pad, C-stick and touch screen to these is input.rs's job.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Space,
    Apostrophe,
    Comma,
    Minus,
    Period,
    Slash,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Semicolon,
    Equal,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    LeftBracket,
    Backslash,
    RightBracket,
    GraveAccent,
    World1,
    World2,
    Escape,
    Enter,
    Tab,
    Backspace,
    Insert,
    Delete,
    Right,
    Left,
    Down,
    Up,
    PageUp,
    PageDown,
    Home,
    End,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    Kp0,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    KpDecimal,
    KpDivide,
    KpMultiply,
    KpSubtract,
    KpAdd,
    KpEnter,
    KpEqual,
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    Menu,
    Back,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left = 0,
    Middle = 1,
    Right = 2,
    Unknown = 255,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyMods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

/// `miniquad::date::now()`: seconds since the epoch as f64, which the game uses for clocks and autosave timing.
pub mod date {
    pub fn now() -> f64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
    }
}

/// `miniquad::window::*` the game calls: no-ops or 3DS equivalents.
pub mod window {
    /// The top screen's drawable size: the game lays its course view out from this.
    pub fn screen_size() -> (f32, f32) {
        (crate::screens::TOP_W as f32, crate::screens::TOP_H as f32)
    }
    pub fn dpi_scale() -> f32 {
        1.0
    }
    pub fn show_mouse(_shown: bool) {}
    pub fn show_keyboard(_shown: bool) {
        // the software keyboard: ctru::applets::swkbd, wired in by input.rs when a text field takes focus
    }
    pub fn order_quit() {
        crate::QUIT.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn set_fullscreen(_on: bool) {}
    pub fn clipboard_get() -> Option<String> {
        None
    }
    pub fn clipboard_set(_s: &str) {}
}
