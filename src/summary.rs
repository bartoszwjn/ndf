use std::fmt;

use crate::attr_path::AttrPath;

pub(crate) struct Summary {
    pub(crate) items: Vec<SummaryItem>,
}

pub(crate) struct SummaryItem {
    pub(crate) base: Option<AttrPath>,
    pub(crate) attr_path: AttrPath,
    pub(crate) result_old: EvalResult,
    pub(crate) result_new: EvalResult,
}

#[derive(Clone, Debug)]
pub(crate) enum EvalResult {
    DrvPath(String),
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EvalResultCmp {
    Equal,
    NotEqual,
    Unknown,
}

impl EvalResult {
    pub(crate) fn compare(&self, other: &Self) -> EvalResultCmp {
        match (self, other) {
            (Self::DrvPath(old), Self::DrvPath(new)) if old == new => EvalResultCmp::Equal,
            (Self::DrvPath(_), Self::DrvPath(_)) => EvalResultCmp::NotEqual,
            (Self::Error, _) | (_, Self::Error) => EvalResultCmp::Unknown,
        }
    }
}

impl EvalResultCmp {
    fn symbol(self) -> &'static str {
        match self {
            EvalResultCmp::Equal => "==",
            EvalResultCmp::NotEqual => "!=",
            EvalResultCmp::Unknown => "??",
        }
    }

    fn display_width(self) -> usize {
        self.symbol().len()
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::styles::HEADER;

        writeln!(f, "{HEADER}Summary:{HEADER:#}")?;
        for item in &self.items {
            writeln!(f, "{item}")?;
        }
        Ok(())
    }
}

impl fmt::Display for SummaryItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result_old = &self.result_old;
        let result_new = &self.result_new;
        let status = result_old.compare(result_new);
        let status_width = status.display_width();

        match &self.base {
            None => {
                let (lhs_pad, rhs_pad) = get_padding(self.attr_path.display_width(), status_width);
                let attr_path = self.attr_path.display();
                writeln!(f, "  {attr_path}{:lhs_pad$} {result_old}", "")?;
                writeln!(f, "  {status}{:rhs_pad$} {result_new}", "")?;
            }
            Some(base) => {
                let (lhs_pad, rhs_pad) =
                    get_padding(base.display_width(), self.attr_path.display_width());
                let base = base.display();
                let attr_path = self.attr_path.display();
                writeln!(
                    f,
                    "  {:status_width$} {base}{:lhs_pad$} {result_old}",
                    "", ""
                )?;
                writeln!(f, "  {status} {attr_path}{:rhs_pad$} {result_new}", "")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for EvalResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::styles::{EVAL_ERROR, EVAL_SUCCESS};
        match self {
            EvalResult::DrvPath(drv_path) => write!(f, "{EVAL_SUCCESS}{drv_path}{EVAL_SUCCESS:#}"),
            EvalResult::Error => write!(f, "{EVAL_ERROR}error{EVAL_ERROR:#}"),
        }
    }
}

impl fmt::Display for EvalResultCmp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::styles::{EQUAL, NOT_EQUAL, UNKNOWN};
        let symbol = self.symbol();
        match self {
            EvalResultCmp::Equal => write!(f, "{EQUAL}{symbol}{EQUAL:#}"),
            EvalResultCmp::NotEqual => write!(f, "{NOT_EQUAL}{symbol}{NOT_EQUAL:#}"),
            EvalResultCmp::Unknown => write!(f, "{UNKNOWN}{symbol}{UNKNOWN:#}"),
        }
    }
}

fn get_padding(lhs_width: usize, rhs_width: usize) -> (usize, usize) {
    let max_width = lhs_width.max(rhs_width);
    (max_width - lhs_width, max_width - rhs_width)
}
