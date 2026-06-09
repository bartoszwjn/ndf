use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Output},
    sync::Mutex,
};

use color_eyre::{Section, SectionExt};
use toml_edit::DocumentMut;

use crate::{
    git::{self, GitRev},
    jj::{self, JjRev},
    nix,
};

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestCase {
    command: CommandConfig,
    repo_contents: RepoContents,
    outputs: Outputs,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandConfig {
    args: Vec<String>,
    current_dir: Option<String>,
    env: Option<HashMap<String, String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum RepoContents {
    Git(GitRepoContents),
    Jj(JjRepoContents),
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GitRepoContents {
    commits: Vec<Commit>,
    index: Option<GitIndex>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JjRepoContents {
    colocate: Option<bool>,
    commits: Vec<Commit>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    message: String,
    parents: Option<Vec<usize>>,
    changes: HashMap<String, Change>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GitIndex {
    parent: Option<usize>,
    changes: HashMap<String, Change>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum Change {
    Write(String),
    Delete,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Outputs {
    exit_code: i32,
    stdout: String,
    stderr: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Common {
    files: HashMap<String, String>,
}

type Substitutions = Vec<(String, String)>;

impl TestCase {
    pub(crate) fn run(name: &str) -> eyre::Result<()> {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("test_cases/{name}"));
        prepare_dir(&dir)?;
        let dir = dir.canonicalize()?;
        eprintln!("running test case in {}", dir.display());

        let mut substitutions = vec![
            ("@SYSTEM@".to_owned(), nix::get_current_system()?),
            ("@REPO@".to_owned(), dir.to_str().unwrap().to_owned()),
        ];

        let common = Common::read(&substitutions)?;

        let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/integration/test_cases/{name}.toml"));
        let test_case = Self::read(&config_path, &substitutions)?;

        match test_case.repo_contents {
            RepoContents::Git(git_repo_contents) => {
                let commits = git_repo_contents.create(&dir, &common)?;
                for (ix, commit) in commits.into_iter().enumerate() {
                    substitutions.extend([
                        (format!("@COMMIT_{ix}_ID@"), commit.id),
                        (format!("@COMMIT_{ix}_SHORT_ID@"), commit.short_id),
                    ]);
                }
            }
            RepoContents::Jj(jj_repo_contents) => {
                let commits = jj_repo_contents.create(&dir, &common)?;
                for (ix, commit) in commits.into_iter().enumerate() {
                    substitutions.extend([
                        (format!("@COMMIT_{ix}_LOG_ONELINE@"), commit.log_oneline),
                        (format!("@COMMIT_{ix}_ID@"), commit.commit_id),
                        (format!("@COMMIT_{ix}_SHORT_ID@"), commit.commit_short_id),
                    ]);
                }
            }
        }

        let output = test_case.command.run(&dir)?;

        if std::env::var_os("NDF_TESTS_UPDATE").is_some_and(|v| !v.is_empty()) {
            test_case
                .outputs
                .update(&output, &substitutions, &config_path)?;
        } else {
            test_case.outputs.check(&output, &substitutions)?;
        }

        Ok(())
    }

    fn read(path: &Path, substitutions: &Substitutions) -> eyre::Result<Self> {
        let mut this: Self = toml::from_slice(&fs::read(path)?)?;
        this.repo_contents.substitute(substitutions);
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
        if let Some(env) = &self.env {
            cmd.envs(env);
        }

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

impl RepoContents {
    fn substitute(&mut self, substitutions: &Substitutions) {
        match self {
            RepoContents::Git(git_repo_contents) => {
                for commit in &mut git_repo_contents.commits {
                    for change in commit.changes.values_mut() {
                        change.substitute(substitutions);
                    }
                }
                if let Some(index) = &mut git_repo_contents.index {
                    for change in index.changes.values_mut() {
                        change.substitute(substitutions);
                    }
                }
            }
            RepoContents::Jj(jj_repo_contents) => {
                for commit in &mut jj_repo_contents.commits {
                    for change in commit.changes.values_mut() {
                        change.substitute(substitutions);
                    }
                }
            }
        }
    }
}

impl GitRepoContents {
    fn create(&self, dir: &Path, common: &Common) -> eyre::Result<Vec<GitRev>> {
        let mut commits = Vec::new();

        git::init(dir)?;
        for (file_name, contents) in &common.files {
            write_file(dir, file_name, contents)?;
        }
        commits.push(git::commit(dir, "initial commit")?);

        for commit in &self.commits {
            if let Some(parents) = &commit.parents {
                assert!(!parents.is_empty());
                if parents.len() == 1 {
                    git::switch(dir, commits[parents[0]].id.as_str())?;
                } else {
                    git::merge(dir, parents.iter().map(|&ix| commits[ix].id.as_str()))?;
                }
            }
            for (file_name, change) in &commit.changes {
                change.apply(dir, file_name)?;
            }
            commits.push(git::commit(dir, &commit.message)?);
        }

        if let Some(index) = &self.index {
            if let Some(parent) = index.parent {
                git::switch(dir, &commits[parent].id)?;
            }
            for (file_name, change) in &index.changes {
                change.apply(dir, file_name)?;
            }
            git::add(dir)?;
        }

        Ok(commits)
    }
}

impl JjRepoContents {
    fn create(&self, dir: &Path, common: &Common) -> eyre::Result<Vec<JjRev>> {
        let mut commits = Vec::new();

        jj::init(dir, self.colocate.unwrap_or(true))?;

        jj::new(dir, "initial commit", [])?;
        for (file_name, contents) in &common.files {
            write_file(dir, file_name, contents)?;
        }
        commits.push(jj::get_rev(dir)?);

        for commit in &self.commits {
            match commit.parents.as_deref() {
                None => jj::new(dir, &commit.message, [])?,
                Some([]) => panic!(
                    "commit {:?} has 0 parents, which is not allowed",
                    commit.message
                ),
                Some(parents @ &[_, ..]) => jj::new(
                    dir,
                    &commit.message,
                    parents.iter().map(|&ix| commits[ix].commit_id.as_str()),
                )?,
            }
            for (file_name, change) in &commit.changes {
                change.apply(dir, file_name)?;
            }
            commits.push(jj::get_rev(dir)?);
        }

        Ok(commits)
    }
}

impl Change {
    fn substitute(&mut self, substitutions: &Substitutions) {
        match self {
            Self::Write(contents) => {
                *contents = substitute_string(std::mem::take(contents), substitutions);
            }
            Self::Delete => {}
        }
    }

    fn apply(&self, dir: &Path, file_name: &str) -> eyre::Result<()> {
        match self {
            Change::Write(contents) => write_file(dir, file_name, contents),
            Change::Delete => fs::remove_file(dir.join(file_name)).map_err(Into::into),
        }
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

        let expected_stdout: &str = &substitute_string(&self.stdout, substitutions);
        pretty_assertions::assert_eq!(expected_stdout, stdout, "unexpected stdout");

        if let Some(expected_stderr) = &self.stderr {
            let expected_stderr: &str = &substitute_string(expected_stderr, substitutions);
            pretty_assertions::assert_eq!(expected_stderr, stderr, "unexpected stderr")
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

        let expected_stdout = substitute_string(&self.stdout, substitutions);
        let stdout = str::from_utf8(&output.stdout).unwrap();
        if stdout != expected_stdout {
            let value = output_config.get_mut("stdout").unwrap();
            *value = reverse_substitutions(stdout, substitutions).into();
            updated = true;
        }

        if let Some(expected_stderr) = &self.stderr {
            let expected_stderr = substitute_string(expected_stderr, substitutions);
            let stderr = str::from_utf8(&output.stderr).unwrap();
            if stderr != expected_stderr {
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
        for file in this.files.values_mut() {
            *file = substitute_string(std::mem::take(file), substitutions);
        }
        Ok(this)
    }
}

fn write_file(dir: &Path, file_name: &str, contents: &str) -> eyre::Result<()> {
    let path = dir.join(file_name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, contents)?;
    Ok(())
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

fn prepare_dir(dir: &Path) -> eyre::Result<()> {
    fs::create_dir_all(dir)?;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}
