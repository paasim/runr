use crate::config::RunConfig;
use crate::err::Result;
use crate::pipeline::{Task, TaskId};
use std::process::ExitStatus;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

#[derive(Debug)]
pub enum WorkInput {
    Task(TaskId, Task),
    Stop,
}

#[derive(Debug)]
pub enum WorkOutput {
    Ok(TaskId),
    Failed(TaskId, String),
}

#[derive(Debug)]
pub struct Worker {
    thread: thread::JoinHandle<()>,
    container_name: Arc<Mutex<Option<String>>>,
}

impl Worker {
    /// Spawn new worker that is able to process tasks
    pub fn new(
        receiver: Arc<Mutex<mpsc::Receiver<WorkInput>>>,
        sender: mpsc::Sender<WorkOutput>,
        config: Arc<RunConfig>,
    ) -> Worker {
        let container_name = Arc::new(Mutex::new(None));
        let in_progress = container_name.clone();
        let thread = thread::spawn(move || {
            loop {
                let (task_id, task) = match receiver.lock().unwrap().recv().unwrap() {
                    WorkInput::Stop => break,
                    WorkInput::Task(task_id, task) => (task_id, task),
                };
                let exit_code = task.run(config.as_ref(), in_progress.clone());
                if !handle_status(exit_code, &task, task_id, &sender) {
                    break;
                }
            }
        });
        Worker {
            thread,
            container_name,
        }
    }

    pub fn join(self) {
        self.thread.join().expect("Couldn't join the thread")
    }

    pub fn container_name(&self) -> Option<String> {
        self.container_name.lock().unwrap().take()
    }
}

/// Send status back and signal of processing new tasks should be stopped
fn handle_status(
    exit_status: Result<ExitStatus>,
    task: &Task,
    task_id: TaskId,
    sender: &mpsc::Sender<WorkOutput>,
) -> bool {
    let code = match exit_status.map(|s| s.code()) {
        Ok(Some(c)) => c,
        Ok(None) => {
            fail(task_id, format!("{task} terminated unexpectedly"), sender);
            return false;
        }
        Err(e) => {
            fail(task_id, format!("{task} exited with an error {e}"), sender);
            return false;
        }
    };
    if code == 0 {
        sender.send(WorkOutput::Ok(task_id)).unwrap();
        return true;
    }
    let msg = format!("{task} exited with error code {code}");
    fail(task_id, msg, sender);
    false
}

fn fail(task_id: TaskId, reason: String, sender: &mpsc::Sender<WorkOutput>) {
    sender.send(WorkOutput::Failed(task_id, reason)).unwrap()
}
