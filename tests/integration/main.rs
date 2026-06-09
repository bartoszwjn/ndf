use std::sync::Once;

use crate::test_case::TestCase;

mod command;
mod git;
mod jj;
mod nix;
mod test_case;

static EYRE_INIT: Once = Once::new();

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
    basic_file,
    basic_flake,
    different_outputs_auto_select_file,
    different_outputs_auto_select_flake,
    different_outputs_manual_select_file,
    different_outputs_manual_select_flake,
    empty_attr_path_file,
    empty_attr_path_flake,
    empty_names_file,
    empty_names_flake,
    empty_repo_file,
    empty_repo_flake,
    file_auto_apply_function,
    file_impure_by_default,
    flake_impure,
    flake_output_lookup,
    flake_pure_by_default,
    flake_search_up,
    manual_selection_file,
    manual_selection_flake,
    nix_diff_custom_args,
    nix_diff_default_args,
    nixos_file,
    nixos_flake,
    nixos_manual_selection_file,
    nixos_manual_selection_flake,
    no_changes_file,
    no_changes_flake,
    no_outputs_file,
    no_outputs_flake,
    not_a_derivation_file,
    not_a_derivation_flake,
    subdir_file,
    subdir_flake,
    weird_names_file,
    weird_names_flake,
    with_base_file,
    with_base_flake,
    // keep-sorted end
);
