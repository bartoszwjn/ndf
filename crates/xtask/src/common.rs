use std::path::Path;

pub(crate) fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_root_is_correct() {
        let root = super::workspace_root();
        for elem in ["Cargo.toml", "src", "crates", "rust-toolchain.toml"] {
            assert!(
                root.join(elem).try_exists().unwrap(),
                "{elem} file not found"
            );
        }
    }
}
