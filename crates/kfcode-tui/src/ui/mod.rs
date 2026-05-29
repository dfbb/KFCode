//! UI utilities: border styles, clipboard access, layout helpers, text selection, and text formatting.
mod border;
mod clipboard;
mod layout;
mod selection;
mod text;

pub use border::{BorderChars, BorderStyle};
pub use clipboard::{Clipboard, ClipboardContent};
pub use layout::*;
pub use selection::Selection;
pub use text::*;
