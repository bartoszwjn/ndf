use crate::diff_spec::AttrPath;

pub(crate) struct Summary {
    pub(crate) items: Vec<SummaryItem>,
}

pub(crate) struct SummaryItem {
    pub(crate) base: Option<AttrPath>,
    pub(crate) attr_path: AttrPath,
    pub(crate) old_drv_path: String,
    pub(crate) new_drv_path: String,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::styles::HEADER;

        writeln!(f, "{HEADER}Summary:{HEADER:#}")?;
        for item in &self.items {
            writeln!(f, "{item}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for SummaryItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::styles::{EQUAL, NOT_EQUAL};

        let status_width = 2_usize;
        let status = if self.old_drv_path == self.new_drv_path {
            format!("{EQUAL}=={EQUAL:#}")
        } else {
            format!("{NOT_EQUAL}!={NOT_EQUAL:#}")
        };

        match &self.base {
            None => {
                let attr_path_width = self.attr_path.display_width();
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
            Some(base) => {
                let lhs_width = base.display_width();
                let rhs_width = self.attr_path.display_width();
                let max_width = lhs_width.max(rhs_width);
                let lhs_pad = max_width - lhs_width;
                let rhs_pad = max_width - rhs_width;
                writeln!(
                    f,
                    "  {:status_width$} {}{:lhs_pad$} {}",
                    "", base, "", self.old_drv_path
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
