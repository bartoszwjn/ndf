//! ANSI text styles used for displaying elements.

use anstyle::{AnsiColor, Style};

pub(crate) const HEADER: Style = Style::new().bold();

pub(crate) const SOURCE: Style = AnsiColor::Blue.on_default();
pub(crate) const ATTR_PATH: Style = AnsiColor::Cyan.on_default();
pub(crate) const ATTR_PATH_QUOTED: Style = AnsiColor::Green.on_default();
pub(crate) const WORKTREE: Style = AnsiColor::Magenta.on_default();

pub(crate) const EQUAL: Style = AnsiColor::Green.on_default();
pub(crate) const NOT_EQUAL: Style = AnsiColor::Red.on_default();

pub(crate) const FROM: Style = AnsiColor::Red.on_default().bold();
pub(crate) const TO: Style = AnsiColor::Green.on_default().bold();
