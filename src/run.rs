use crate::config::RunConfig;
use crate::container_command::kill_container;
use crate::err::Result;
use crate::pipeline::{Task, TaskId};
use crate::status::Status;
use crate::worker::{WorkInput, WorkOutput, Worker};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex, mpsc};
use std::{fmt, io};

#[derive(Debug)]
pub struct Run {
    status: Status,
    workers: Vec<Worker>,
    sender: mpsc::Sender<WorkInput>,
    receiver: mpsc::Receiver<WorkOutput>,
    tasks: HashMap<TaskId, Task>,
}

impl Run {
    /// Initialize run by setting up communication channels and initializing the workers.
    pub fn new(n_workers: usize, config: RunConfig, tasks: HashMap<TaskId, Task>) -> Self {
        let config = Arc::new(config);
        let (task_sender, task_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        let task_receiver = Arc::new(Mutex::new(task_receiver));
        let workers = (0..n_workers)
            .map(|_| Worker::new(task_receiver.clone(), result_sender.clone(), config.clone()))
            .collect();
        let deps = tasks.iter().map(|(id, t)| (*id, t.depends())).collect();
        Self {
            status: Status::new(deps),
            workers,
            sender: task_sender,
            receiver: result_receiver,
            tasks,
        }
    }

    pub fn is_completed(&self) -> bool {
        self.status.is_completed()
    }

    pub fn is_succeeded(&self) -> bool {
        self.status.is_succeeded()
    }

    /// Check for new output
    pub fn check_output(&self) -> Result<WorkOutput> {
        Ok(self.receiver.recv()?)
    }

    /// Submit all runnable tasks
    pub fn submit_runnable(&mut self) -> Result<()> {
        while let Some(task_id) = self.status.next_runnable() {
            let Some(task) = self.tasks.remove(&task_id) else {
                return Err(io::Error::other("Inconsistent run status"))?;
            };
            self.sender.send(WorkInput::Task(task_id, task))?
        }
        Ok(())
    }

    /// Start the run
    pub fn start(&mut self) -> Result<()> {
        while !self.status.is_completed() {
            self.submit_runnable()?;
            match self.check_output()? {
                WorkOutput::Ok(id) => self.status.complete(id, true),
                WorkOutput::Failed(id, s) => {
                    eprintln!("{s}\nKilling containers and exiting.");
                    self.status.complete(id, false);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// Cleanup afterwards, send stop signal and wait for all processes to stop.
    ///
    /// In case of a failed job, also kill running containers.
    pub fn cleanup(mut self) -> Result<usize> {
        let (output_reader, output) = io::pipe()?;
        let stop_handles: Result<Vec<_>> = self
            .workers
            .drain(..)
            .map(|w| {
                if let Err(e) = self.sender.send(WorkInput::Stop) {
                    eprintln!("error with sending stop signal: {e}");
                };
                Ok((
                    w.container_name()
                        .map(|n| kill_container(n, output.try_clone()?)),
                    w,
                ))
            })
            .collect();
        drop(output);
        let mut killed_sub = 0;
        for (container, worker) in stop_handles? {
            if let Some(container) = container {
                container?.wait()?;
                killed_sub += 1;
            }
            worker.join();
        }
        for line in BufReader::new(output_reader).lines() {
            println!("{}", line?);
        }
        Ok(killed_sub)
    }
}

impl fmt::Display for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.status)
    }
}
