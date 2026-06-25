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
    file_auto_apply_function,
    file_basic,
    file_different_outputs_auto_select,
    file_different_outputs_manual_select,
    file_empty_attr_path,
    file_empty_names,
    file_empty_repo,
    file_impure_by_default,
    file_leading_dot,
    file_manual_selection,
    file_nixos,
    file_nixos_manual_selection,
    file_no_changes,
    file_no_outputs,
    file_not_a_derivation,
    file_subdir,
    file_weird_names,
    file_with_base,
    flake_basic,
    flake_default_outputs_sorting,
    flake_different_outputs_auto_select,
    flake_different_outputs_manual_select,
    flake_empty_attr_path,
    flake_empty_names,
    flake_empty_repo,
    flake_impure,
    flake_manual_selection,
    flake_nixos,
    flake_nixos_manual_selection,
    flake_no_changes,
    flake_no_outputs,
    flake_not_a_derivation,
    flake_output_lookup,
    flake_pure_by_default,
    flake_search_up,
    flake_subdir,
    flake_weird_names,
    flake_with_base,
    git_in_jj_repo,
    git_with_from,
    git_with_from_and_to,
    git_with_revision,
    git_with_revision_and_base,
    git_with_revision_first_parent,
    git_with_to,
    git_with_working_tree_revision,
    glob_file,
    glob_file_leading_dot,
    glob_flake,
    glob_flake_nixos_output_lookup,
    glob_flake_output_lookup,
    glob_outputs_sorting,
    glob_outputs_union,
    jj_file_basic,
    jj_flake_basic,
    jj_in_git_repo,
    jj_non_colocated_repo,
    jj_with_from,
    jj_with_from_and_to,
    jj_with_revision,
    jj_with_revision_and_base,
    jj_with_revision_first_parent,
    jj_with_to,
    jj_with_working_tree_revision,
    nix_diff_custom_args,
    nix_diff_default_args,
    // keep-sorted end
);
