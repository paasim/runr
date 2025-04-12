use crate::pipeline::{TaskId, TaskIds};
use std::fmt;

#[derive(Debug)]
pub struct Status {
    new: Vec<(TaskId, TaskIds)>,
    in_progress: TaskIds,
    completed: TaskIds,
}

impl Status {
    /// Set the given `task_id` to be completed.
    pub fn complete(&mut self, task_id: TaskId) {
        let ids = TaskIds::from(task_id);
        self.in_progress &= !ids;
        self.completed |= ids;
    }

    /// Check if the entire run is completed.
    pub fn is_completed(&self) -> bool {
        self.new.is_empty() && self.in_progress.is_empty()
    }

    /// Initialize a new run given the task ids and their dependencies.
    /// Assumes none of the tasks are in progress or completed.
    pub fn new(deps: Vec<(TaskId, TaskIds)>) -> Self {
        Self {
            new: deps,
            in_progress: TaskIds::default(),
            completed: TaskIds::default(),
        }
    }

    /// Query for a next runnable task (ie. a task that has all of its dependencies completed).
    pub fn next_runnable(&mut self) -> Option<TaskId> {
        let incompl = !self.completed;
        let ind = self.new.iter().position(|t| (t.1 & incompl).is_empty())?;
        let id = self.new.swap_remove(ind).0;
        self.in_progress |= TaskIds::from(id);
        Some(id)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.new.is_empty() {
            let new_ids: TaskIds = self.new.iter().map(|x| x.0).collect();
            writeln!(f, "Unstarted tasks: {new_ids}")?;
        }
        if !self.in_progress.is_empty() {
            writeln!(f, "Ongoing tasks:   {}", self.in_progress)?;
        }
        if !self.completed.is_empty() {
            write!(f, "Completed tasks: {}", self.completed)?;
        }
        if self.is_completed() {
            write!(f, " (done)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn status_works() {
        let tasks = [
            [3].into_iter()
                .map(|i| TaskId::try_from(i).unwrap())
                .collect(),
            [0, 2]
                .into_iter()
                .map(|i| TaskId::try_from(i).unwrap())
                .collect(),
            TaskIds::default(),
            TaskIds::default(),
        ];
        let tasks = tasks
            .into_iter()
            .enumerate()
            .map(|(i, ids)| (TaskId::try_from(i).unwrap(), ids))
            .collect();
        let mut status = Status::new(tasks);

        // Two tasks can be started (as they have no dependencies).
        assert_eq!(status.next_runnable(), Some(TaskId::try_from(2).unwrap()));
        assert_eq!(status.next_runnable(), Some(TaskId::try_from(3).unwrap()));
        assert_eq!(status.next_runnable(), None);
        assert!(!status.is_completed());

        status.complete(TaskId::try_from(2).unwrap());
        // still no new tasks cannot be started
        assert_eq!(status.next_runnable(), None);
        status.complete(TaskId::try_from(3).unwrap());

        // task 2 completed => 0 can be started and the run can be completed
        assert_eq!(status.next_runnable(), Some(TaskId::try_from(0).unwrap()));
        assert_eq!(status.next_runnable(), None);
        assert!(!status.is_completed());
        status.complete(TaskId::try_from(0).unwrap());

        // do final task
        assert!(status.next_runnable().is_some());
        status.complete(TaskId::try_from(1).unwrap());

        // done
        assert!(status.is_completed());
    }
}
