use serde::{Serialize, Deserialize};

/// A keyboard key event, representing a virtual key code
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum KeyEvent {
    /// The insert key
    Insert,

    /// The home key
    Home,

    /// The delete key
    Delete,

    /// The end key
    End,

    /// The page down key
    PageDown,

    /// The page up key
    PageUp,

    /// The left arrow key
    Left,

    /// The up arrow key
    Up,

    /// The right arrow key
    Right,

    /// The down arrow key
    Down,

    /// The backspace button
    Back,

    /// The enter or return key
    Return,

    /// The spacebar
    Space,

    /// The escape key
    Escape,

    /// The tab key
    Tab,

    /// Function key 1
    F1,

    /// Function key 2
    F2,

    /// Function key 3
    F3,

    /// Function key 4
    F4,

    /// Function key 5
    F5,

    /// Function key 6
    F6,

    /// Function key 7
    F7,

    /// Function key 8
    F8,

    /// Function key 9
    F9,

    /// Function key 10
    F10,

    /// Function key 11
    F11,

    /// Function key 12
    F12,
}