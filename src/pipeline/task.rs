use super::TaskIds;
use crate::config::RunConfig;
use crate::container_command::ContainerCommand;
use crate::err::Result;
use std::io::{BufRead, BufReader, PipeWriter, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::{fmt, io};

const SHELL: &str = "/bin/bash";

#[derive(Clone, Debug, PartialEq)]
pub enum Task {
    CommandLine {
        name: String,
        commands: String,
        depends: TaskIds,
    },
    Container {
        name: String,
        commands: String,
        image: String,
        depends: TaskIds,
    },
    PullImage(String),
}

impl Task {
    pub fn command(
        name: String,
        commands: String,
        image: Option<String>,
        depends: TaskIds,
    ) -> Self {
        if let Some(image) = image {
            return Self::Container {
                name,
                commands,
                image,
                depends,
            };
        }
        Self::CommandLine {
            name,
            commands,
            depends,
        }
    }

    pub fn depends(&self) -> TaskIds {
        match self {
            Task::CommandLine { depends, .. } => *depends,
            Task::Container { depends, .. } => *depends,
            Task::PullImage(_) => TaskIds::default(),
        }
    }

    pub fn name_width(&self) -> Option<usize> {
        match self {
            Task::CommandLine { name, .. } => Some(name.len()),
            Task::Container { name, .. } => Some(name.len()),
            Task::PullImage(_) => None,
        }
    }

    /// Run the task.
    /// `CommandLine`-task sets `in_progress` to the container name while the process is running.
    pub fn run(
        &self,
        config: &RunConfig,
        in_progress: Arc<Mutex<Option<String>>>,
    ) -> Result<ExitStatus> {
        let (output_reader, output) = io::pipe()?;
        match self {
            Task::CommandLine { name, commands, .. } => {
                let mut child = spawn_cmd(output, config.repo_path())?;
                write!(child.stdin.take().expect("run stdin taken"), "{commands}")?;
                let width = config.name_width();
                for line in BufReader::new(output_reader).lines() {
                    println!("{name:width$}| {}", line?);
                }
                let status = child.wait()?;
                Ok(status)
            }
            Task::Container {
                name,
                commands,
                image,
                ..
            } => {
                let container_name = &config.mk_container_name(name);
                let cmd = ContainerCommand::Run {
                    commands,
                    image,
                    container_name,
                    config,
                };
                *in_progress.lock().unwrap() = Some(container_name.clone());
                let mut child = cmd.start(output)?;
                let width = config.name_width();
                for line in BufReader::new(output_reader).lines() {
                    println!("{name:width$}| {}", line?);
                }
                let status = child.wait()?;
                *in_progress.lock().unwrap() = None;
                Ok(status)
            }
            Task::PullImage(img) => {
                let mut child = ContainerCommand::Pull(img).start(output)?;
                for line in BufReader::new(output_reader).lines() {
                    println!("{}", line?);
                }
                Ok(child.wait()?)
            }
        }
    }
}

fn spawn_cmd(output: PipeWriter, repo_path: &Path) -> Result<Child> {
    Ok(Command::new(SHELL)
        .current_dir(repo_path)
        .stdout(output.try_clone()?)
        .stderr(output)
        .stdin(Stdio::piped())
        .spawn()?)
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (cmd, name) = match self {
            Task::CommandLine { name, .. } => ("shell", name),
            Task::Container { name, .. } => ("container", name),
            Task::PullImage(name) => ("pull", name),
        };
        write!(f, "{cmd:10} {name}")
    }
}
