use anstream::println;

use crate::{
    cli::DiffProgram,
    color::{GREEN_BOLD, RED_BOLD},
    command::Cmd,
    diff_spec::{AttrPath, DiffSpec},
    eval::EvalSpec,
    summary::SummaryItem,
};

pub(crate) fn compare_paths<'spec>(
    attr_path: &'spec AttrPath,
    spec: &'spec DiffSpec,
    mut get_drv_path: impl FnMut(EvalSpec<'spec>) -> anyhow::Result<String>,
) -> anyhow::Result<SummaryItem> {
    let attr_path_l = spec.common_lhs.as_ref().unwrap_or(attr_path);
    let lhs_spec = EvalSpec::lhs(spec, attr_path_l);
    let rhs_spec = EvalSpec::rhs(spec, attr_path);
    let old_drv_path = get_drv_path(lhs_spec)?;
    let new_drv_path = get_drv_path(rhs_spec)?;

    match spec.program {
        DiffProgram::None => {}
        DiffProgram::NixDiff => {
            if old_drv_path != new_drv_path {
                print_pair_cmp(lhs_spec, rhs_spec);
                run_nix_diff(&old_drv_path, &new_drv_path)?;
                println!();
            }
        }
        DiffProgram::Nvd => todo!("nvd diff"),
    }

    Ok(SummaryItem {
        common_lhs: spec.common_lhs.clone(),
        attr_path: attr_path.clone(),
        old_drv_path,
        new_drv_path,
    })
}

fn print_pair_cmp(lhs: EvalSpec, rhs: EvalSpec) {
    let width_l = unicode_width::UnicodeWidthStr::width(lhs.attr_path.0.as_str());
    let width_r = unicode_width::UnicodeWidthStr::width(rhs.attr_path.0.as_str());
    let width = width_l.max(width_r);
    let lhs_pad = width - width_l;
    let rhs_pad = width - width_r;
    println!(
        "{RED_BOLD}-{RED_BOLD:#} {}{:lhs_pad$} {}",
        lhs.attr_path, "", lhs.git_rev
    );
    println!(
        "{GREEN_BOLD}+{GREEN_BOLD:#} {}{:rhs_pad$} {}",
        rhs.attr_path, "", rhs.git_rev
    );
}

fn run_nix_diff(old_drv_path: &str, new_drv_path: &str) -> anyhow::Result<()> {
    Cmd::nix_diff()
        .args([
            "--character-oriented",
            "--skip-already-compared",
            old_drv_path,
            new_drv_path,
        ])
        .run_inherit_stdio()
}
