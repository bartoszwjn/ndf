use std::sync::Once;

use crate::test_case::TestCase;

mod git;
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
    dirty_worktree_file,
    dirty_worktree_flake,
    empty_attr_path_file,
    empty_attr_path_flake,
    empty_names_file,
    empty_names_flake,
    empty_repo_file,
    empty_repo_flake,
    file_auto_apply_function,
    flake_output_lookup,
    manual_selection_file,
    manual_selection_flake,
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
    search_up_for_flake,
    subdir_file,
    subdir_flake,
    weird_names_file,
    weird_names_flake,
    // keep-sorted end
);
