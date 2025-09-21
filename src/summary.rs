use crate::{
    color::{BOLD, GREEN, YELLOW},
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
        let status = if self.old_drv_path == self.new_drv_path {
            format!("{GREEN}=={GREEN:#}")
        } else {
            format!("{YELLOW}!={YELLOW:#}")
        };
        let lhs = self.common_lhs.as_ref().unwrap_or(&self.attr_path);

        writeln!(f, "     {} {}", lhs, self.old_drv_path)?;
        writeln!(f, "  {status} {} {}", self.attr_path, self.new_drv_path)?;
        Ok(())
    }
}
