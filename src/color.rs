use anstyle::{AnsiColor, Style};

// keep-sorted start
pub(crate) const BLUE: Style = AnsiColor::Blue.on_default();
pub(crate) const CYAN: Style = AnsiColor::Cyan.on_default();
pub(crate) const GREEN: Style = AnsiColor::Green.on_default();
pub(crate) const MAGENTA: Style = AnsiColor::Magenta.on_default();
pub(crate) const RED: Style = AnsiColor::Red.on_default();
pub(crate) const YELLOW: Style = AnsiColor::Yellow.on_default();
// keep-sorted end

pub(crate) const BOLD: Style = Style::new().bold();

// keep-sorted start
pub(crate) const RED_BOLD: Style = AnsiColor::Red.on_default().bold();
// keep-sorted end
