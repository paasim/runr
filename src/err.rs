use crate::pipeline::TaskId;
use crate::worker::WorkInput;
use std::{error, fmt, io, num, sync::mpsc};

#[derive(Debug)]
pub enum Error {
    DependencyCycle(Vec<String>),
    DuplicateTask(String),
    FailedTask(TaskId, String),
    Io(io::Error),
    TooManyTasks(usize),
    UndefinedTask(String),
    Worker(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DependencyCycle(tasks) => {
                write!(f, "Dependency cycle containing the following tasks:")?;
                tasks.iter().try_for_each(|t| write!(f, "\n  - {t}"))
            }
            Error::DuplicateTask(n) => write!(f, "Task {n} defined multiple times"),
            Error::FailedTask(i, e) => write!(f, "Task [{i}] failed:\n{e}"),
            Error::Io(e) => write!(f, "{e}"),
            Error::TooManyTasks(n) => write!(f, "Too many ({n} > 255) tasks + images"),
            Error::UndefinedTask(tn) => write!(f, "Undefined task name '{tn}'"),
            Error::Worker(e) => write!(f, "{e}"),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<num::TryFromIntError> for Error {
    fn from(value: num::TryFromIntError) -> Self {
        Self::Io(io::Error::other(value))
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Io(io::Error::other(value))
    }
}

impl From<mpsc::SendError<WorkInput>> for Error {
    fn from(mpsc::SendError(msg): mpsc::SendError<WorkInput>) -> Self {
        match msg {
            WorkInput::Task(task_id, task) => {
                Self::Worker(format!("Can't submit task [{task_id}] {task}"))
            }
            WorkInput::Stop => Self::Worker("Cant' send stop message".to_string()),
        }
    }
}

impl From<mpsc::RecvError> for Error {
    fn from(_: mpsc::RecvError) -> Self {
        Self::Worker("Can't receive results from workers".to_string())
    }
}

impl<T: fmt::Display> From<(TaskId, T)> for Error {
    fn from((task_id, err): (TaskId, T)) -> Self {
        Self::FailedTask(task_id, err.to_string())
    }
}
