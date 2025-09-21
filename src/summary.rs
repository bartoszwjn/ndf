use unicode_width::UnicodeWidthStr;

use crate::{
    color::{BOLD, GREEN, RED},
    spec::AttrPath,
};

pub(crate) struct Summary {
    pub(crate) items: Vec<SummaryItem>,
}

pub(crate) struct SummaryItem {
    pub(crate) common_lhs: Option<AttrPath>,
    pub(crate) attr_path: AttrPath,
    pub(crate) old_drv_path: String,
    pub(crate) new_drv_path: String,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{BOLD}Summary:{BOLD:#}")?;
        for item in &self.items {
            writeln!(f, "{item}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for SummaryItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_width = 2_usize;
        let status = if self.old_drv_path == self.new_drv_path {
            format!("{GREEN}=={GREEN:#}")
        } else {
            format!("{RED}!={RED:#}")
        };

        match &self.common_lhs {
            None => {
                let attr_path_width = UnicodeWidthStr::width(self.attr_path.0.as_str());
                let max_width = attr_path_width.max(2);
                let lhs_pad = max_width - attr_path_width;
                let rhs_pad = max_width - status_width;
                writeln!(
                    f,
                    "  {}{:lhs_pad$} {}",
                    self.attr_path, "", self.old_drv_path
                )?;
                writeln!(f, "  {}{:rhs_pad$} {}", status, "", self.new_drv_path)?;
            }
            Some(lhs) => {
                let lhs_width = UnicodeWidthStr::width(lhs.0.as_str());
                let rhs_width = UnicodeWidthStr::width(self.attr_path.0.as_str());
                let max_width = lhs_width.max(rhs_width);
                let lhs_pad = max_width - lhs_width;
                let rhs_pad = max_width - rhs_width;
                writeln!(
                    f,
                    "  {:status_width$} {}{:lhs_pad$} {}",
                    "", lhs, "", self.old_drv_path
                )?;
                writeln!(
                    f,
                    "  {} {}{:rhs_pad$} {}",
                    status, self.attr_path, "", self.new_drv_path
                )?;
            }
        }

        Ok(())
    }
}
