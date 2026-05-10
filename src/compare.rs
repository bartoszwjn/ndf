use crate::{
    attr_path::AttrPath,
    cli::DiffTool,
    command::Cmd,
    diff_spec::DiffSpec,
    eval::EvalSpec,
    summary::{EvalResult, SummaryItem},
};

pub(crate) fn compare_paths<'spec>(
    attr_path: &'spec AttrPath,
    spec: &'spec DiffSpec,
    mut eval: impl FnMut(EvalSpec<'spec>) -> EvalResult,
) -> eyre::Result<SummaryItem> {
    let lhs_spec = EvalSpec::lhs(spec, attr_path);
    let rhs_spec = EvalSpec::rhs(spec, attr_path);
    let result_old = eval(lhs_spec);
    let result_new = eval(rhs_spec);

    match spec.tool {
        DiffTool::None => {}
        DiffTool::NixDiff => {
            if let EvalResult::DrvPath(old_drv_path) = &result_old
                && let EvalResult::DrvPath(new_drv_path) = &result_new
                && old_drv_path != new_drv_path
            {
                print_pair_cmp(lhs_spec.attr_path, rhs_spec.attr_path, spec);
                run_nix_diff(old_drv_path, new_drv_path)?;
                anstream::println!();
            }
        }
    }

    Ok(SummaryItem {
        base: spec.base.clone(),
        attr_path: attr_path.clone(),
        result_old,
        result_new,
    })
}

fn print_pair_cmp(lhs: &AttrPath, rhs: &AttrPath, spec: &DiffSpec) {
    let width_l = lhs.display_width();
    let width_r = rhs.display_width();
    let width = width_l.max(width_r);
    let lhs_pad = width - width_l;
    let rhs_pad = width - width_r;
    let DiffSpec { from, to, .. } = spec;

    use crate::styles::{FROM, TO};
    anstream::println!("{FROM}-{FROM:#} {}{:lhs_pad$} {from}", lhs.display(), "");
    anstream::println!("{TO}+{TO:#} {}{:rhs_pad$} {to}", rhs.display(), "");
}

fn run_nix_diff(old_drv_path: &str, new_drv_path: &str) -> eyre::Result<()> {
    Cmd::nix_diff()
        .args([
            "--character-oriented",
            "--skip-already-compared",
            old_drv_path,
            new_drv_path,
        ])
        .run_inherit_stdio()
}
