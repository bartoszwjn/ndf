use crate::{
    cli::DiffTool,
    command::Cmd,
    diff_spec::{AttrPath, DiffSpec},
    eval::EvalSpec,
    summary::SummaryItem,
};

pub(crate) fn compare_paths<'spec>(
    attr_path: &'spec AttrPath,
    spec: &'spec DiffSpec,
    mut get_drv_path: impl FnMut(EvalSpec<'spec>) -> eyre::Result<String>,
) -> eyre::Result<SummaryItem> {
    let lhs_spec = EvalSpec::lhs(spec, attr_path);
    let rhs_spec = EvalSpec::rhs(spec, attr_path);
    let old_drv_path = get_drv_path(lhs_spec)?;
    let new_drv_path = get_drv_path(rhs_spec)?;

    match spec.tool {
        DiffTool::None => {}
        DiffTool::NixDiff => {
            if old_drv_path != new_drv_path {
                print_pair_cmp(lhs_spec.attr_path, rhs_spec.attr_path, spec);
                run_nix_diff(&old_drv_path, &new_drv_path)?;
                anstream::println!();
            }
        }
    }

    Ok(SummaryItem {
        base: spec.base.clone(),
        attr_path: attr_path.clone(),
        old_drv_path,
        new_drv_path,
    })
}

fn print_pair_cmp(lhs: &AttrPath, rhs: &AttrPath, spec: &DiffSpec) {
    let width_l = unicode_width::UnicodeWidthStr::width(lhs.0.as_str());
    let width_r = unicode_width::UnicodeWidthStr::width(rhs.0.as_str());
    let width = width_l.max(width_r);
    let lhs_pad = width - width_l;
    let rhs_pad = width - width_r;
    let DiffSpec { from, to, .. } = spec;

    use crate::styles::{FROM, TO};
    anstream::println!("{FROM}-{FROM:#} {lhs}{:lhs_pad$} {from}", "");
    anstream::println!("{TO}+{TO:#} {rhs}{:rhs_pad$} {to}", "");
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
