use crate::err::Result;
use crate::pipeline::Pipeline;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io};

const YAML: &str = "runr.yaml";

#[derive(Debug)]
pub struct Config {
    bare_path: PathBuf,
    repo_name: String,
    repo_branch: String,
    default_image: Option<String>,
    pipeline_filename: String,
    timestamp: u64,
    cleanup: bool,
}

impl Config {
    /// Read config from environment variables.
    ///
    /// Relevant variables are
    /// * `BARE_PATH`: path to the bare repository, defaults to [env::current_dir()]
    /// * `BRANCH`: *WARNING*: panics if unset
    /// * `DEFAULT_IMAGE`: default image to be used if the image is unset, defaults to
    ///   debian:bookworm
    /// * `PIPELINE_FILENAME`: filename for the pipeline definition, defaults to `runr.yaml`
    /// * `CLEANUP`: should the containers and directories be removed, defaults to `true`
    pub fn from_env() -> Self {
        let bare_path: PathBuf = match env::var("BARE_PATH") {
            Ok(p) => p.parse().expect("Invalid value for BARE_PATH"),
            Err(_) => env::current_dir().expect("Unable to obtain current dir"),
        };
        let repo_name = match bare_path.file_name().and_then(|s| s.to_str()) {
            Some(p) => p.to_string(),
            None => panic!("Invalid value for BARE_PATH"),
        };
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(t) => t.as_secs(),
            Err(e) => panic!("Invalid system time {e}"),
        };
        let cleanup = match env::var("CLEANUP") {
            Ok(c) => c.parse().expect("Invalid value for CLEANUP"),
            _ => true,
        };
        Self {
            bare_path,
            repo_name,
            repo_branch: env::var("BRANCH").expect("BRANCH missing"),
            default_image: env::var("DEFAULT_IMAGE").ok(),
            pipeline_filename: env::var("PIPELINE_FILENAME").unwrap_or(YAML.to_string()),
            timestamp,
            cleanup,
        }
    }

    pub fn pipeline_filename(&self) -> PathBuf {
        let mut path = self.repo_path();
        path.push(&self.pipeline_filename);
        path
    }

    /// Path to clone the repository to.
    pub fn repo_path(&self) -> PathBuf {
        let mut checkout_path = env::temp_dir();
        checkout_path.push(&self.repo_name);
        checkout_path.push(&self.repo_branch);
        checkout_path.push(self.timestamp.to_string());
        checkout_path
    }

    pub fn default_image(&self) -> &Option<String> {
        &self.default_image
    }

    pub fn run_config(&self, pipeline: &Pipeline) -> RunConfig {
        let container_name_prefix = format!(
            "runr-{}-{}-{}",
            self.repo_name, self.repo_branch, self.timestamp
        );
        RunConfig::new(
            self.repo_path(),
            container_name_prefix,
            self.cleanup,
            pipeline.name_width(),
        )
    }

    /// Remove the cloned repository.
    pub fn cleanup(&self) -> Result<()> {
        if self.cleanup {
            fs::remove_dir_all(self.repo_path())?;
        }
        Ok(())
    }
}

pub fn repo_checkout(config: &Config) -> Result<()> {
    let repo_path = config.repo_path();
    let status_clone = Command::new("git")
        .args(["clone", "-q"])
        .args([&config.bare_path, &repo_path])
        .status()?;
    if !status_clone.success() {
        Err(io::Error::other("unable to clone"))?
    }
    let status_checkout = Command::new("git")
        .current_dir(repo_path)
        .env_remove("GIT_DIR")
        .args(["checkout", "-q", &config.repo_branch])
        .status()?;
    if !status_checkout.success() {
        Err(io::Error::other("unable to checkout"))?
    }
    Ok(())
}

/// Part of the configuration that is relevant during runtime.
#[derive(Debug)]
pub struct RunConfig {
    repo_path: PathBuf,
    container_name_prefix: String,
    cleanup: bool,
    task_name_width: usize,
}

impl RunConfig {
    pub fn new(
        repo_path: PathBuf,
        container_name_prefix: String,
        cleanup: bool,
        task_name_width: usize,
    ) -> Self {
        Self {
            repo_path,
            container_name_prefix,
            cleanup,
            task_name_width,
        }
    }

    pub fn cleanup(&self) -> bool {
        self.cleanup
    }

    pub fn mk_container_name(&self, task_name: &str) -> String {
        let Ok(ts) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            // TODO
            todo!()
        };
        format!(
            "{}-{task_name}-{}",
            self.container_name_prefix,
            ts.as_secs()
        )
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn name_width(&self) -> usize {
        self.task_name_width
    }
}
