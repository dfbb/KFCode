//! Border character sets and style variants for drawing TUI panels.

/// The six characters used to draw a rectangular border.
pub struct BorderChars {
    /// Top-left corner character.
    pub top_left: char,
    /// Top-right corner character.
    pub top_right: char,
    /// Bottom-left corner character.
    pub bottom_left: char,
    /// Bottom-right corner character.
    pub bottom_right: char,
    /// Horizontal line character.
    pub horizontal: char,
    /// Vertical line character.
    pub vertical: char,
}

impl Clone for BorderChars {
    fn clone(&self) -> Self {
        Self {
            top_left: self.top_left,
            top_right: self.top_right,
            bottom_left: self.bottom_left,
            bottom_right: self.bottom_right,
            horizontal: self.horizontal,
            vertical: self.vertical,
        }
    }
}

impl Default for BorderChars {
    fn default() -> Self {
        Self::rounded()
    }
}

impl BorderChars {
    /// Rounded corners using box-drawing characters (╭╮╰╯).
    pub fn rounded() -> Self {
        Self {
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
            horizontal: '─',
            vertical: '│',
        }
    }

    /// Square corners using standard box-drawing characters (┌┐└┘).
    pub fn squared() -> Self {
        Self {
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
            horizontal: '─',
            vertical: '│',
        }
    }

    /// Heavy/thick border using double-weight box-drawing characters (┏┓┗┛).
    pub fn thick() -> Self {
        Self {
            top_left: '┏',
            top_right: '┓',
            bottom_left: '┗',
            bottom_right: '┛',
            horizontal: '━',
            vertical: '┃',
        }
    }

    /// Invisible border using space characters.
    pub fn empty() -> Self {
        Self {
            top_left: ' ',
            top_right: ' ',
            bottom_left: ' ',
            bottom_right: ' ',
            horizontal: ' ',
            vertical: ' ',
        }
    }

    /// A minimal border with only a vertical bar and a custom bottom-left corner.
    pub fn custom(vertical: char, bottom_left: char) -> Self {
        Self {
            top_left: ' ',
            top_right: ' ',
            bottom_left,
            bottom_right: ' ',
            horizontal: ' ',
            vertical,
        }
    }
}

/// High-level border style selector.
#[derive(Clone)]
pub enum BorderStyle {
    /// No border drawn.
    None,
    /// Single-line border.
    Single,
    /// Double-line border.
    Double,
    /// Rounded-corner border.
    Rounded,
    /// Heavy/thick border.
    Thick,
    /// Fully custom character set.
    Custom(BorderChars),
}
