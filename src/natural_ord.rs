use std::cmp::Ordering;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub(crate) struct NaturalOrdStr<'a>(pub(crate) &'a str);

impl PartialOrd for NaturalOrdStr<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NaturalOrdStr<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(self.0, other.0)
    }
}

fn compare(lhs: &str, rhs: &str) -> Ordering {
    // `impl Ord for str` also works on byte values
    // https://doc.rust-lang.org/std/primitive.str.html#impl-Ord-for-str
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    let mut l_iter = lhs.iter().copied().zip(0_usize..);
    let mut r_iter = rhs.iter().copied().zip(0_usize..);
    loop {
        let (l_val, l_ix, r_val, r_ix) = match (l_iter.next(), r_iter.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some((l_val, l_ix)), Some((r_val, r_ix))) => (l_val, l_ix, r_val, r_ix),
        };

        let cmp = if l_val.is_ascii_digit() && r_val.is_ascii_digit() {
            let (l_zeros, l_number, l_rest) = parse_number(&lhs[l_ix..]);
            let (r_zeros, r_number, r_rest) = parse_number(&rhs[r_ix..]);
            l_iter = l_rest.iter().copied().zip((lhs.len() - l_rest.len())..);
            r_iter = r_rest.iter().copied().zip((rhs.len() - r_rest.len())..);

            // We're comparing numbers without leading zeros, so the longer number is greater.
            (l_number.len().cmp(&r_number.len()))
                // If the numbers have the same length, then we can compare them lexicographically.
                .then_with(|| l_number.cmp(r_number))
                // If numbers have the same numerical value, order them by number of leading zeros.
                .then_with(|| l_zeros.len().cmp(&r_zeros.len()))
        } else {
            l_val.cmp(&r_val)
        };

        if cmp.is_ne() {
            return cmp;
        }
    }
}

fn parse_number(bytes: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let zeros_end = bytes.iter().position(|b| *b != b'0').unwrap_or(bytes.len());
    let (leading_zeros, rest) = bytes.split_at(zeros_end);
    let digits_end = rest
        .iter()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(rest.len());
    let (digits, rest) = rest.split_at(digits_end);
    (leading_zeros, digits, rest)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    fn assert_total_order(sequence: &[&str]) {
        for (i, lhs) in sequence.iter().copied().enumerate() {
            for (j, rhs) in sequence.iter().copied().enumerate() {
                let expected = i.cmp(&j);
                let result = super::compare(lhs, rhs);
                let symbol = |ordering| match ordering {
                    Ordering::Less => '<',
                    Ordering::Equal => '=',
                    Ordering::Greater => '>',
                };
                assert_eq!(
                    expected,
                    result,
                    "expected {lhs:?} {} {rhs:?}, got {lhs:?} {} {rhs:?}",
                    symbol(expected),
                    symbol(result),
                );
            }
        }
    }

    #[test]
    fn total_order() {
        assert_total_order(&[
            "", "\0", ".", ".foo", "0", "00", "0000", "1", "01", "0001", "2", "3", "3_", "3_1",
            "3_tail", "4", "20", "21", "199", "abc", "d1", "d01", "d2", "d3_e1", "d3_e2", "d3_e12",
            "d3_f1", "d9", "d10", "d32", "z",
        ])
    }
}
