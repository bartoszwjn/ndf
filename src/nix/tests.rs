#[test]
fn to_string_literal() {
    let cases = [
        // simple strings
        ("", r#""""#),
        ("abc", r#""abc""#),
        // `"` and `\`
        (r#"""#, r#""\"""#),
        (r#"\"#, r#""\\""#),
        (r#""" and "foo" and """#, r#""\"\" and \"foo\" and \"\"""#),
        (r#"\n \r \t"#, r#""\\n \\r \\t""#),
        // `$` and `${`
        ("$", r#""$""#),
        ("${", r#""\${""#),
        ("with $var", r#""with $var""#),
        ("like ${bash} :)", r#""like \${bash} :)""#),
        // `\n`, `\r`, `\t`
        ("\n", r#""\n""#),
        ("\r", r#""\r""#),
        ("\t", r#""\t""#),
        ("hello\nthre\rthere\tbye", r#""hello\nthre\rthere\tbye""#),
    ];

    for case in cases {
        let result = super::to_string_literal(case.0).to_string();
        assert_eq!(
            result, case.1,
            "{:?}: unexpected result of to_string_literal",
            case.0,
        );
    }
}
