use std::sync::Once;

use crate::test_case::TestCase;

mod git;
mod nix;
mod test_case;

static EYRE_INIT: Once = Once::new();

#[allow(dead_code)]
fn run_test_case(name: &str) -> eyre::Result<()> {
    EYRE_INIT.call_once(|| color_eyre::install().unwrap());
    TestCase::run(name)
}

macro_rules! test_cases {
    ($($name:ident,)*) => {
        $(
            #[test]
            fn $name() -> eyre::Result<()> {
                run_test_case(stringify!($name))
            }
        )*
    };
}

test_cases!(
    // keep-sorted start
    // keep-sorted end
);
