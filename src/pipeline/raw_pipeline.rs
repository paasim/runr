use super::raw_task::RawTask;
use super::{Task, TaskId, TaskIds};
use crate::err::Result;
use crate::pipeline::task_name::TaskNames;
use std::collections::{HashMap, hash_map};
use std::num::NonZeroUsize;
use std::thread;

/// Pipeline that is simply read from the input as is.
/// This is further checked for undefined dependencies, cycles etc. and
/// transformed into [super::Pipeline] that is run.
#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct RawPipeline {
    default_image: Option<String>,
    n_parallel: Option<usize>,
    tasks: Vec<RawTask>,
}

impl RawPipeline {
    /// Defaults to [thread::available_parallelism()] if set to zero.
    pub fn n_parallel(&self) -> Result<NonZeroUsize> {
        match self.n_parallel {
            None => Ok(NonZeroUsize::MIN), //1
            // zero results to the number of threads
            Some(n) => Ok(NonZeroUsize::new(n).unwrap_or(thread::available_parallelism()?)),
        }
    }

    /// Obtain [Task]s.
    ///
    /// The difference to [RawTask]s is that each task gets assigned a unique
    /// [TaskId] which are used for dependencies (instead of task names).
    /// In addition, missing images are replaced with default values.
    pub fn tasks(self, default_image: &str) -> Result<HashMap<TaskId, Task>> {
        let default_image = self.default_image.as_deref().unwrap_or(default_image);
        let id_map = TaskNames::from_tasks(&self.tasks, default_image)?;
        let mut tasks = HashMap::new();
        for task in self.tasks.iter() {
            let mut depends = TaskIds::default();
            for dep in task.depends.as_deref().unwrap_or_default() {
                depends |= TaskIds::from(id_map.get_task_id(dep)?);
            }
            let image_name = task.image.as_deref().unwrap_or(default_image);
            let image_id = id_map.get_image_id(image_name)?;
            depends |= TaskIds::from(image_id);
            // only add image if its not already added
            if let hash_map::Entry::Vacant(e) = tasks.entry(image_id) {
                e.insert(Task::PullImage(image_name.to_owned()));
            }

            let id = id_map.get_task_id(&task.name)?;
            let task = Task::CommandLine {
                name: task.name.to_owned(),
                commands: task.commands.to_owned(),
                image: image_name.to_owned(),
                depends,
            };
            tasks.insert(id, task);
        }
        Ok(tasks)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_empty() {
        let yaml = "tasks:";
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        let raw_tasks_exp = RawPipeline {
            default_image: None,
            n_parallel: None,
            tasks: vec![],
        };
        assert_eq!(raw_tasks, raw_tasks_exp)
    }

    #[test]
    fn parse_two_raw_tasks() {
        let yaml = r#"
        default_image: "default-image"
        tasks:
        - commands: |
            echo
            exit 0
          name: "step-1"
          depends: []
        - image: "image0"
          commands: echo n
          name: n
          depends:
            - "step-0"
            - "step-1"
        n_parallel: 77
        "#;
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();

        let task1 = RawTask {
            name: String::from("step-1"),
            commands: String::from("echo\nexit 0\n"),
            image: None,
            depends: Some(vec![]),
        };
        let task2 = RawTask {
            name: String::from("n"),
            commands: String::from("echo n"),
            image: Some(String::from("image0")),
            depends: Some(vec![String::from("step-0"), String::from("step-1")]),
        };
        let tasks_exp = RawPipeline {
            default_image: Some(String::from("default-image")),
            n_parallel: Some(77),
            tasks: vec![task1, task2],
        };
        assert_eq!(raw_tasks, tasks_exp)
    }

    #[test]
    fn invalid_deps() {
        let yaml = r#"
        tasks:
        - commands: cmd
          name: "step-1"
          depends: ["step-2"]
        "#;
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        assert!(raw_tasks.tasks("image").is_err());
    }

    #[test]
    fn duplicate_names() {
        let yaml = r#"
        tasks:
        - commands: cmd
          name: "step-1"
        - commands: cmd2
          name: "step-1"
        "#;
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        assert!(raw_tasks.tasks("imagez").is_err());
    }
}
