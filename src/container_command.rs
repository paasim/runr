use crate::config::RunConfig;
use crate::err::Result;
use std::io::{PipeWriter, Write};
use std::process::{Child, Command, Stdio};

/// Commands that are run using `podman`.
/// Other options could be supported in the future.
#[derive(Debug)]
pub enum ContainerCommand<'a> {
    Run {
        commands: &'a str,
        image: &'a str,
        container_name: &'a str,
        config: &'a RunConfig,
    },
    Pull(&'a str),
    Kill(&'a str),
}

impl<'a> ContainerCommand<'a> {
    /// Start running the command.
    /// The resulting `Child`-object must be waited (or killed) to ensure it finishes.
    pub fn start(self, output: PipeWriter) -> Result<Child> {
        match self {
            ContainerCommand::Run {
                commands,
                config,
                image,
                container_name,
            } => {
                let mut child = spawn_container(container_name, image, config, output)?;
                write!(child.stdin.take().expect("run stdin taken"), "{commands}")?;
                Ok(child)
            }
            ContainerCommand::Pull(name) => Ok(Command::new("podman")
                .args(["pull", name])
                .stdout(output.try_clone()?)
                .stderr(output)
                .spawn()?),
            ContainerCommand::Kill(name) => Ok(Command::new("podman")
                .args(["kill", name])
                .stdout(output.try_clone()?)
                .stderr(output)
                .spawn()?),
        }
    }
}

fn spawn_container(
    name: &str,
    image_name: &str,
    config: &RunConfig,
    output: PipeWriter,
) -> Result<Child> {
    let run_args = match config.cleanup() {
        true => ["run", "--rm", "--interactive", "--userns", "keep-id"].as_slice(),
        false => ["run", "--interactive", "--userns", "keep-id"].as_slice(),
    };
    let workdir = "/__repo";
    let repo_path = config.repo_path().to_str().expect("invalid repo path");
    let volume = format!("{repo_path}:{workdir}");
    Ok(Command::new("podman")
        .args(run_args)
        .args(["--name", name, "--volume", &volume, "--workdir", workdir])
        .args([image_name, "/bin/bash"])
        .stdout(output.try_clone()?)
        .stderr(output)
        .stdin(Stdio::piped())
        .spawn()?)
}

pub fn kill_container(name: String, output: PipeWriter) -> Result<Child> {
    ContainerCommand::Kill(&name).start(output)
}
