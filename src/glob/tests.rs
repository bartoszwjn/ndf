use super::{BracketExpr, ParseError, Part, Pattern, Segment};

type Segments<'a> = &'a [&'a [Segment]];

fn mk(leading_dot: bool, segments: Segments) -> Pattern {
    Pattern {
        leading_dot,
        parts: segments
            .iter()
            .map(|part| Part {
                segments: part.to_vec(),
            })
            .collect(),
    }
}

fn lit(s: &str) -> Segment {
    Segment::Literal(s.to_owned())
}

fn star() -> Segment {
    Segment::Star
}

fn qm() -> Segment {
    Segment::QuestionMark
}

fn bre(negated: bool, content: &str) -> Segment {
    Segment::BracketExpr(BracketExpr {
        negated,
        content: content.to_owned(),
    })
}

#[test]
fn parse_pattern() -> eyre::Result<()> {
    let cases: [(&str, (bool, Segments)); _] = [
        // simple
        ("a", (false, &[&[lit("a")]])),
        (
            "foo.bar.baz",
            (false, &[&[lit("foo")], &[lit("bar")], &[lit("baz")]]),
        ),
        // leading dot
        (".", (true, &[])),
        (".foo", (true, &[&[lit("foo")]])),
        (".foo.bar", (true, &[&[lit("foo")], &[lit("bar")]])),
        // wildcards
        ("*", (false, &[&[star()]])),
        ("?", (false, &[&[qm()]])),
        ("foo*bar", (false, &[&[lit("foo"), star(), lit("bar")]])),
        ("foo?bar", (false, &[&[lit("foo"), qm(), lit("bar")]])),
        (
            "foo.*.bar",
            (false, &[&[lit("foo")], &[star()], &[lit("bar")]]),
        ),
        (
            "foo.?.bar",
            (false, &[&[lit("foo")], &[qm()], &[lit("bar")]]),
        ),
        // bracket exprs
        ("[a]", (false, &[&[bre(false, "a")]])),
        ("[abc]", (false, &[&[bre(false, "abc")]])),
        ("[^abc]", (false, &[&[bre(true, "abc")]])),
        ("[!abc]", (false, &[&[bre(true, "abc")]])),
        ("[a-z]", (false, &[&[bre(false, "a-z")]])),
        ("[[:alpha:]]", (false, &[&[bre(false, "[:alpha:]")]])),
        ("[[.ch.]]", (false, &[&[bre(false, "[.ch.]")]])),
        ("[[=a=]]", (false, &[&[bre(false, "[=a=]")]])),
        (
            "[abc[:alpha:][.ch.][=a=]x-z-]",
            (false, &[&[bre(false, "abc[:alpha:][.ch.][=a=]x-z-")]]),
        ),
        ("ba[rz]", (false, &[&[lit("ba"), bre(false, "rz")]])),
        (
            "foo.[abc].bar",
            (false, &[&[lit("foo")], &[bre(false, "abc")], &[lit("bar")]]),
        ),
        // quoting
        (r#""foo""#, (false, &[&[lit("foo")]])),
        (r#"f"o"o"#, (false, &[&[lit("foo")]])),
        (r#"".foo""#, (false, &[&[lit(".foo")]])),
        (r#""foo.bar.baz""#, (false, &[&[lit("foo.bar.baz")]])),
        (
            r#"foo."bar.baz".quux"#,
            (false, &[&[lit("foo")], &[lit("bar.baz")], &[lit("quux")]]),
        ),
        ("", (false, &[])),
        (r#""""#, (false, &[&[]])),
        (
            r#"foo."".bar"#,
            (false, &[&[lit("foo")], &[], &[lit("bar")]]),
        ),
        (r#""foo**?[bar]""#, (false, &[&[lit("foo**?[bar]")]])),
    ];

    for (input, expected) in cases {
        let parsed = super::parse_pattern(input)?;
        let expected = mk(expected.0, expected.1);
        assert_eq!(
            expected, parsed,
            "{input:?}: unexpected result of parse_pattern",
        );
    }
    Ok(())
}

#[test]
fn parse_pattern_errors() {
    let cases: [(&str, ParseError); _] = [
        ("..", ParseError::ConsecutiveDots),
        ("..foo.bar", ParseError::ConsecutiveDots),
        ("foo..bar", ParseError::ConsecutiveDots),
        ("foo.bar..", ParseError::ConsecutiveDots),
        ("foo.", ParseError::TrailingDot),
        (r#"""#, ParseError::NoClosingQuote),
        (r#"foo."bar"#, ParseError::NoClosingQuote),
        (r#""foo".bar.baz""#, ParseError::NoClosingQuote),
        ("**", ParseError::ConsecutiveStars),
        ("foo**", ParseError::ConsecutiveStars),
        ("foo**", ParseError::ConsecutiveStars),
        ("[abc", ParseError::UnclosedBracketExpr),
        ("[]", ParseError::UnclosedBracketExpr),
        ("[^]", ParseError::UnclosedBracketExpr),
        ("[[:alpha]", ParseError::UnclosedCharacterClass),
        ("[[:alpha:", ParseError::UnclosedCharacterClass),
        ("[[=a]", ParseError::UnclosedEquivalenceClass),
        ("[[=a=", ParseError::UnclosedEquivalenceClass),
        ("[[.ch]", ParseError::UnclosedCollatingSymbol),
        ("[[.ch.", ParseError::UnclosedCollatingSymbol),
    ];

    for (input, expected) in cases {
        let result = super::parse_pattern(input);
        assert_eq!(
            Err(expected),
            result,
            "{input:?}: unexpected result of parse_pattern",
        );
    }
}
