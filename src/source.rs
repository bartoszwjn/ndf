use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use eyre::{WrapErr, bail};

/// A source of Nix expressions.
#[derive(Clone, Debug)]
pub(crate) enum Source {
    Flake(FlakePath),
    /// Absolute, canonicalized path to the file.
    File(PathBuf),
}

impl Source {
    pub(crate) fn flake(flake_ref: &str) -> eyre::Result<Self> {
        let path = Path::new(flake_ref);
        // Based on the path-like syntax described in `nix flake --help`,
        // but we allow relative paths to start with `../` as well for convenience.
        //
        // https://nix.dev/manual/nix/2.33/command-ref/new-cli/nix3-flake.html#path-like-syntax
        //
        // Nix doesn't actually require relative paths to start with `./`,
        // a `.` anywhere inside the path is enough for it not to be treated as a `flake:` shorthand,
        // so in practice Nix allows this as well.
        if !(path.is_absolute() || path.starts_with(".") || path.starts_with("..")) {
            bail!("flake paths must be absolute paths, or start with './' or '../'");
        }
        if flake_ref.contains(['#', '?']) {
            bail!("flake paths must not contain '#' or '?' characters");
        }

        let path = path
            .canonicalize()
            .wrap_err_with(|| format!("failed to resolve path {path:?}"))?;
        let metadata = path
            .metadata()
            .wrap_err_with(|| format!("failed to query metadata of {path:?}"))?;
        if !metadata.is_dir() {
            bail!("{path:?} is not a directory");
        }

        let mut flake_path = path.clone();
        loop {
            flake_path.push("flake.nix");
            let has_flake_nix = flake_path
                .try_exists()
                .wrap_err_with(|| format!("failed to check for existence of {flake_path:?}"))?;
            flake_path.pop();
            if has_flake_nix {
                return Ok(Self::Flake(FlakePath::new(flake_path)?));
            }

            flake_path.push(".git");
            let has_dot_git = flake_path
                .try_exists()
                .wrap_err_with(|| format!("failed to check for existence of {flake_path:?}"))?;
            flake_path.pop();
            if has_dot_git || &flake_path == "/" {
                bail!(
                    "path {path:?} is not part of a flake \
                (neither it nor its parent directories contain a 'flake.nix' file)"
                );
            }

            flake_path.pop();
        }
    }

    pub(crate) fn file(file_path: &OsStr) -> eyre::Result<Self> {
        let s = file_path.to_string_lossy();

        if file_path.is_empty() {
            return Err(eyre::eyre!("empty paths are not supported"));
        }
        // Reject special forms accepted by `nix`'s `--file` option.
        // https://docs.lix.systems/manual/lix/2.94/command-ref/nix-build.html#fileish-syntax
        if s.starts_with('<') && s.ends_with('>') {
            return Err(eyre::eyre!(
                "search paths (paths surrounded by '<' and '>') are not supported"
            ));
        }
        for prefix in ["http://", "https://", "flake:", "channel:"] {
            if s.starts_with(prefix) {
                return Err(eyre::eyre!(
                    "paths starting with {prefix:?} are not supported"
                ));
            }
        }

        let mut absolute = fs::canonicalize(file_path)
            .wrap_err_with(|| format!("failed to resolve path {file_path:?}"))?;
        let mut metadata = fs::metadata(&absolute)
            .wrap_err_with(|| format!("failed to query metadata of {absolute:?}"))?;

        if metadata.file_type().is_dir() {
            absolute.push("default.nix");
            metadata = fs::metadata(&absolute)
                .wrap_err(format!("failed to query metadata of {absolute:?}"))?;
        }

        if metadata.file_type().is_dir() {
            bail!("{absolute:?} is a directory");
        }

        Ok(Self::File(absolute))
    }
}

/// Absolute, canonicalized path to the directory containing the `flake.nix` file.
///
/// Guaranteed to contain only characters that can be used in path-like flake references.
#[derive(Clone, Debug)]
pub(crate) struct FlakePath(String);

impl FlakePath {
    pub(crate) fn new(path: PathBuf) -> eyre::Result<Self> {
        assert!(path.is_absolute());
        let string = match path.into_os_string().into_string() {
            Ok(string) => string,
            Err(os_string) => bail!("flake path contains invalid Unicode: {os_string:?}"),
        };
        if let Some(invalid) = string.chars().find(|&c| !Self::is_valid_char(c)) {
            bail!(
                "flake path contains an invalid character: {}",
                invalid.escape_default(),
            )
        }
        Ok(Self(string))
    }

    fn is_valid_char(c: char) -> bool {
        // Nix allows all unicode characters except `#` and `?`, but Lix is more restrictive:
        // https://nix.dev/manual/nix/2.33/command-ref/new-cli/nix3-flake.html#path-like-syntax
        // https://git.lix.systems/lix-project/lix/src/commit/2.94.0/lix/libexpr/flake/flakeref.cc#L86
        //
        // TODO: it should be possible to express any path using URL-like syntax
        // with percent encoding.
        c.is_ascii_alphanumeric() || "-._~!$&'\"()*+,;=/".contains(c)
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_ref()
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}
