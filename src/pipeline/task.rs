use super::TaskIds;
use crate::config::RunConfig;
use crate::container_command::ContainerCommand;
use crate::err::Result;
use std::io::{BufRead, BufReader};
use std::process::ExitStatus;
use std::sync::{Arc, Mutex};
use std::{fmt, io};

#[derive(Clone, Debug, PartialEq)]
pub enum Task {
    CommandLine {
        name: String,
        commands: String,
        image: String,
        depends: TaskIds,
    },
    PullImage(String),
}

impl Task {
    pub fn depends(&self) -> TaskIds {
        match self {
            Task::CommandLine { depends, .. } => *depends,
            Task::PullImage(_) => TaskIds::default(),
        }
    }

    pub fn name_width(&self) -> Option<usize> {
        match self {
            Task::CommandLine { name, .. } => Some(name.len()),
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
            Task::CommandLine {
                name,
                commands,
                image,
                ..
            } => {
                let container_name = &config.container_name(name);
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

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Task::CommandLine { name, .. } => write!(f, "{name}"),
            Task::PullImage(name) => write!(f, "pull {name}"),
        }
    }
}
