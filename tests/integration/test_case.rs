use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Output},
    sync::Mutex,
};

use color_eyre::{Section, SectionExt};
use tempfile::TempDir;
use toml_edit::DocumentMut;

use crate::{
    git::{self, GitRev},
    nix,
};

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TestCase {
    command: CommandConfig,
    repo_contents: GitRepoContents,
    outputs: Outputs,
}

#[derive(Debug, serde::Deserialize)]
struct CommandConfig {
    args: Vec<String>,
    current_dir: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GitRepoContents {
    commits: Vec<GitCommit>,
    index: Option<GitIndex>,
}

#[derive(Debug, serde::Deserialize)]
struct GitCommit {
    message: String,
    files: Files,
}

#[derive(Debug, serde::Deserialize)]
struct GitIndex {
    files: Files,
}

#[derive(Debug, serde::Deserialize)]
struct Outputs {
    exit_code: i32,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Common {
    files: Files,
}

type Files = HashMap<String, String>;
type Substitutions = Vec<(String, String)>;

impl TestCase {
    pub(crate) fn run(name: &str) -> eyre::Result<()> {
        let dir = TempDir::with_prefix(format!("ndf_test_{name}_"))?;
        let dir_path = dir.path().canonicalize()?;
        eprintln!("running test case in {}", dir_path.display());

        let mut substitutions = vec![
            ("@SYSTEM@".to_owned(), nix::get_current_system()?),
            ("@REPO@".to_owned(), dir_path.to_str().unwrap().to_owned()),
        ];

        let common = Common::read(&substitutions)?;

        let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/integration/test_cases/{name}.toml"));
        let test_case = Self::read(&config_path, &substitutions)?;

        let commits = test_case.repo_contents.create(&dir_path, &common.files)?;
        for (commit_ix, commit) in commits.into_iter().enumerate() {
            substitutions.push((format!("@COMMIT_{commit_ix}_ID@"), commit.id));
            substitutions.push((format!("@COMMIT_{commit_ix}_SHORT_ID@"), commit.short_id));
        }

        let output = test_case.command.run(&dir_path)?;

        if std::env::var_os("NDF_TESTS_UPDATE").is_some_and(|v| !v.is_empty()) {
            test_case
                .outputs
                .update(&output, &substitutions, &config_path)?;
        } else {
            test_case.outputs.check(&output, &substitutions)?;
        }

        dir.close()?;
        Ok(())
    }

    fn read(path: &Path, substitutions: &Substitutions) -> eyre::Result<Self> {
        let mut this: Self = toml::from_slice(&fs::read(path)?)?;

        for commit in &mut this.repo_contents.commits {
            substitute_files(&mut commit.files, substitutions);
        }
        if let Some(index) = &mut this.repo_contents.index {
            substitute_files(&mut index.files, substitutions);
        }

        Ok(this)
    }
}

impl CommandConfig {
    fn run(&self, dir: &Path) -> eyre::Result<Output> {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_ndf"));
        let current_dir = if let Some(p) = &self.current_dir {
            &dir.join(p).canonicalize()?
        } else {
            dir
        };
        cmd.current_dir(current_dir).args(&self.args);

        let output = {
            // Running too many instances of Lix (as of 2.95.2) in parallel causes
            // `SQLite database '.../fetcher-cache-v1.sqlite' is busy` errors.
            // Running only one instance of `ndf` at a time manages to avoid that.
            static SINGLE_RUN: Mutex<()> = Mutex::new(());
            let _guard = SINGLE_RUN.lock().unwrap();
            cmd.output()?
        };

        Ok(output)
    }
}

impl GitRepoContents {
    fn create(&self, dir: &Path, common_files: &Files) -> eyre::Result<Vec<GitRev>> {
        let mut commits = Vec::new();

        commits.push(git::init(dir)?);
        for commit in &self.commits {
            Self::clear_dir(dir)?;
            for (file_name, contents) in common_files.iter().chain(commit.files.iter()) {
                Self::write_file(dir, file_name, contents)?;
            }
            commits.push(git::commit(dir, &commit.message)?);
        }
        if let Some(index) = &self.index {
            Self::clear_dir(dir)?;
            for (file_name, contents) in common_files.iter().chain(index.files.iter()) {
                Self::write_file(dir, file_name, contents)?;
            }
            git::add(dir)?;
        }

        Ok(commits)
    }

    fn clear_dir(dir: &Path) -> eyre::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_name() != ".git" {
                if entry.file_type()?.is_dir() {
                    fs::remove_dir_all(entry.path())?;
                } else {
                    fs::remove_file(entry.path())?;
                }
            }
        }
        Ok(())
    }

    fn write_file(dir: &Path, file_name: &str, contents: &str) -> eyre::Result<()> {
        let path = dir.join(file_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        Ok(())
    }
}

impl Outputs {
    fn check(&self, output: &Output, substitutions: &Substitutions) -> eyre::Result<()> {
        let exit_code = output.status.code().unwrap();
        let stdout: &str = &String::from_utf8_lossy(&output.stdout);
        let stderr: &str = &String::from_utf8_lossy(&output.stderr);

        if exit_code != self.exit_code {
            return Err(eyre::eyre!(
                "unexpected exit code ({}), expected {}",
                exit_code,
                self.exit_code
            )
            .section(stdout.to_owned().header("Captured stdout:"))
            .section(stderr.to_owned().header("Captured stderr:")));
        }

        if let Some(expected) = &self.stdout {
            let expected: &str = &substitute_string(expected, substitutions);
            pretty_assertions::assert_eq!(expected, stdout, "unexpected stdout");
        }

        if let Some(expected) = &self.stderr {
            let expected: &str = &substitute_string(expected, substitutions);
            pretty_assertions::assert_eq!(expected, stderr, "unexpected stderr")
        }

        Ok(())
    }

    fn update(
        &self,
        output: &Output,
        substitutions: &Substitutions,
        config_path: &Path,
    ) -> eyre::Result<()> {
        let mut config = fs::read_to_string(config_path)?.parse::<DocumentMut>()?;
        let mut updated = false;
        let output_config = config.get_mut("outputs").unwrap();

        let exit_code = output.status.code().unwrap();
        if exit_code != self.exit_code {
            let value = output_config
                .get_mut("exit_code")
                .unwrap()
                .as_value_mut()
                .unwrap();
            *value = i64::from(exit_code).into();
            updated = true;
        }

        if let Some(expected) = &self.stdout {
            let expected = substitute_string(expected, substitutions);
            let stdout = str::from_utf8(&output.stdout).unwrap();
            if stdout != expected {
                let value = output_config.get_mut("stdout").unwrap();
                *value = reverse_substitutions(stdout, substitutions).into();
                updated = true;
            }
        }

        if let Some(expected) = &self.stderr {
            let expected = substitute_string(expected, substitutions);
            let stderr = str::from_utf8(&output.stderr).unwrap();
            if stderr != expected {
                let value = output_config.get_mut("stderr").unwrap();
                *value = reverse_substitutions(stderr, substitutions).into();
                updated = true;
            }
        }

        if updated {
            fs::write(config_path, config.to_string())?;
        }

        Ok(())
    }
}

impl Common {
    fn read(substitutions: &Substitutions) -> eyre::Result<Self> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/integration/test_cases/common.toml");
        let mut this: Self = toml::from_slice(&fs::read(&path)?)?;
        substitute_files(&mut this.files, substitutions);
        Ok(this)
    }
}

fn substitute_files(files: &mut Files, substitutions: &Substitutions) {
    for file in files.values_mut() {
        *file = substitute_string(std::mem::take(file), substitutions);
    }
}

fn substitute_string(s: impl Into<String>, substitutions: &Substitutions) -> String {
    let mut s = s.into();
    for (var, value) in substitutions {
        s = s.replace(var, value);
    }
    s
}

fn reverse_substitutions(s: impl Into<String>, substitutions: &Substitutions) -> String {
    let mut s = s.into();
    for (var, value) in substitutions {
        s = s.replace(value, var);
    }
    s
}
