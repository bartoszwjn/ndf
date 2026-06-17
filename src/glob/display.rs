use std::fmt;

use super::{BracketExpr, Part, Pattern, Segment};

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::styles::{PATTERN, PATTERN_EMPTY};

        if self.leading_dot {
            write!(f, "{PATTERN}.{PATTERN:#}")?;
        } else if self.parts.is_empty() {
            write!(f, "{PATTERN_EMPTY}(empty){PATTERN_EMPTY:#}")?;
        }

        let mut parts = self.parts.iter().peekable();
        while let Some(part) = parts.next() {
            write!(f, "{part}")?;
            if parts.peek().is_some() {
                write!(f, "{PATTERN}.{PATTERN:#}")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::styles::PATTERN_QUOTED;

        if self.segments.is_empty() {
            write!(f, "{PATTERN_QUOTED}\"\"{PATTERN_QUOTED:#}")
        } else {
            for segment in &self.segments {
                write!(f, "{segment}")?;
            }
            Ok(())
        }
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::styles::{PATTERN, PATTERN_QUESTION_MARK, PATTERN_QUOTED, PATTERN_STAR};

        match self {
            Segment::Literal(literal) => {
                if literal_needs_quotes(literal) {
                    write!(f, "{PATTERN_QUOTED}\"{literal}\"{PATTERN_QUOTED:#}")
                } else {
                    write!(f, "{PATTERN}{literal}{PATTERN:#}")
                }
            }
            Segment::QuestionMark => write!(f, "{PATTERN_QUESTION_MARK}?{PATTERN_QUESTION_MARK:#}"),
            Segment::Star => write!(f, "{PATTERN_STAR}*{PATTERN_STAR:#}"),
            Segment::BracketExpr(bracket_expr) => write!(f, "{bracket_expr}"),
        }
    }
}

impl fmt::Display for BracketExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::styles::{
            PATTERN as P, PATTERN_BRACKET_EXPR as BE, PATTERN_BRACKET_EXPR_NEGATED as BEN,
        };

        let content = &self.content;
        if self.negated {
            write!(f, "{BEN}[^{BEN:#}{P}{content}{P:#}{BEN}]{BEN:#}")
        } else {
            write!(f, "{BE}[{BE:#}{P}{content}{P:#}{BE}]{BE:#}")
        }
    }
}

fn literal_needs_quotes(literal: &str) -> bool {
    literal.is_empty()
        || literal.contains(['.', '"', '*', '?', '[', ']'])
        || literal.contains(char::is_whitespace)
}
