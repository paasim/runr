use crate::config::{Config, RunConfig};
use crate::err::{Error, Result};
use crate::run::Run;
use raw_pipeline::RawPipeline;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::num::NonZeroUsize;
pub use task::Task;
pub use task_id::{TaskId, TaskIds};

mod raw_pipeline;
mod raw_task;
mod task;
mod task_id;
mod task_name;

#[derive(Debug, PartialEq)]
pub struct Pipeline {
    n_parallel: NonZeroUsize,
    tasks: HashMap<TaskId, Task>,
}

impl Pipeline {
    fn from_raw(raw_pipeline: RawPipeline, default_image: &str) -> Result<Self> {
        let n_parallel = raw_pipeline.n_parallel()?;
        let tasks = raw_pipeline.tasks(default_image)?;
        if let Some(task_ids) = check_cycles(&tasks) {
            let names = task_ids.ids().map(|i| tasks[&i].to_string()).collect();
            return Err(Error::DependencyCycle(names));
        }
        Ok(Self { tasks, n_parallel })
    }

    /// Max width for prepending task name to stdout
    pub fn name_width(&self) -> usize {
        let min_default = 10;
        let min_tasks = self.tasks.values().filter_map(|t| t.name_width()).min();
        (min_tasks.unwrap_or(0) + 2).min(min_default)
    }

    pub fn read_from(rdr: impl Read, default_image: &str) -> Result<Self> {
        Self::from_raw(
            serde_yaml::from_reader::<_, RawPipeline>(rdr)?,
            default_image,
        )
    }

    pub fn run(self, config: RunConfig) -> Run {
        Run::new(self.n_parallel.get(), config, self.tasks)
    }
}

/// Read the pipeline and validate it (no cycles, all dependencies exist etc).
pub fn read_pipeline(config: &Config) -> Result<Pipeline> {
    let file = File::open(config.pipeline_filename())?;
    let default_image = config.default_image();
    Pipeline::read_from(file, default_image)
}

/// Simply run DFS to check for cycles, if any task(id) leads back to itself
/// in the dependency graph, then we have a cycle.
fn check_cycles(tasks: &HashMap<TaskId, Task>) -> Option<TaskIds> {
    let mut visited = TaskIds::default();
    tasks
        .keys()
        .copied()
        .find_map(|i| visit(i, &mut visited, &mut TaskIds::default(), tasks))
}

fn visit(
    task_id: TaskId,
    checked: &mut TaskIds,
    checking: &mut TaskIds,
    tasks: &HashMap<TaskId, Task>,
) -> Option<TaskIds> {
    let task_ids = TaskIds::from(task_id);
    if !(*checked & task_ids).is_empty() {
        return None; // already checked so we can skip this
    }
    if !(*checking & task_ids).is_empty() {
        return Some(*checking); // cycle
    }
    *checking |= task_ids;
    let deps = tasks[&task_id].depends();
    if let Some(i) = deps.ids().find_map(|d| visit(d, checked, checking, tasks)) {
        return Some(i);
    }
    *checking &= !task_ids;
    *checked |= task_ids;
    None
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::available_parallelism;

    #[test]
    fn parse_empty_pipeline() {
        let yaml = "tasks:";
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        let pipeline = Pipeline::from_raw(raw_tasks, "").unwrap();
        assert_eq!(pipeline.tasks, HashMap::new());
        assert_eq!(pipeline.n_parallel, NonZeroUsize::new(1).unwrap());
    }

    #[test]
    fn parse_two_task_pipeline() {
        let yaml = r#"
        n_parallel: 9
        tasks:
        - commands: |
            echo
            exit 0
          name: "step-1"
          depends: []
        - image: "image0"
          commands: echo n
          name: n
          depends: ["step-1"]
        "#;
        let img0 = "DEFAULT";
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        let pipeline = Pipeline::from_raw(raw_tasks, img0).unwrap();

        let task0 = Task::PullImage(String::from(img0));
        let task1 = Task::CommandLine {
            name: String::from("step-1"),
            commands: String::from("echo\nexit 0\n"),
            image: String::from(img0),
            depends: [0]
                .into_iter()
                .map(|i| TaskId::try_from(i).unwrap())
                .collect(),
        };
        let task2 = Task::PullImage(String::from("image0"));
        let task3 = Task::CommandLine {
            name: String::from("n"),
            commands: String::from("echo n"),
            image: String::from("image0"),
            depends: [1, 2]
                .into_iter()
                .map(|i| TaskId::try_from(i).unwrap())
                .collect(),
        };
        let tasks = [task0, task1, task2, task3]
            .into_iter()
            .enumerate()
            .map(|(i, t)| (TaskId::try_from(i).unwrap(), t))
            .collect();
        assert_eq!(pipeline.tasks, tasks);
        assert_eq!(pipeline.n_parallel, NonZeroUsize::new(9).unwrap());
    }

    #[test]
    fn two_task_pipeline_with_default_image() {
        let yaml = r#"
        default_image: img77
        n_parallel: 0
        tasks:
        - commands: |
            echo
            exit 0
          name: "step-1"
          depends: []
        - commands: echo n
          name: n
          depends: ["step-1"]
        "#;
        let img = "IMAGE";
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        let pipeline = Pipeline::from_raw(raw_tasks, img).unwrap();

        let task0 = Task::PullImage(String::from("img77"));
        let task1 = Task::CommandLine {
            name: String::from("step-1"),
            commands: String::from("echo\nexit 0\n"),
            image: String::from("img77"),
            depends: [0]
                .into_iter()
                .map(|i| TaskId::try_from(i).unwrap())
                .collect(),
        };
        let task2 = Task::CommandLine {
            name: String::from("n"),
            commands: String::from("echo n"),
            image: String::from("img77"),
            depends: [0, 1]
                .into_iter()
                .map(|i| TaskId::try_from(i).unwrap())
                .collect(),
        };

        let tasks = vec![task0, task1, task2]
            .into_iter()
            .enumerate()
            .map(|(i, t)| (TaskId::try_from(i).unwrap(), t))
            .collect();
        assert_eq!(pipeline.tasks, tasks);
        assert_eq!(pipeline.n_parallel, available_parallelism().unwrap());
    }

    #[test]
    fn cycle1() {
        let yaml = r#"
        tasks:
        - commands: cmd
          name: "self"
          depends: ["self"]
        "#;
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        assert!(Pipeline::from_raw(raw_tasks, "img").is_err());
    }

    #[test]
    fn longer_cycle() {
        let yaml = r#"
        tasks:
        - commands: cmd1
          name: "step-1"
          depends: ["step-2"]
        - commands: cmd2
          name: step-2
          depends: ["step-3"]
        - commands: cmd3
          name: step-3
          depends: ["step-4", "step-5"]
        - commands: cmd5
          name: step-5
        - commands: cmd4
          name: step-4
          depends: ["step-1"]
        "#;
        let raw_tasks: RawPipeline = serde_yaml::from_str(yaml).unwrap();
        assert!(Pipeline::from_raw(raw_tasks, "img").is_err());
    }
}
