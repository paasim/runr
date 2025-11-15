use super::TaskId;
use super::raw_task::RawTask;
use crate::err::{Error, Result};
use std::collections::{HashMap, hash_map};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum TaskName<'a> {
    Task(&'a str),
    Image(&'a str),
}

/// Used for obtaining unique identifiers for each [super::Task].
#[derive(Debug)]
pub struct TaskNames<'a>(HashMap<TaskName<'a>, TaskId>);

impl<'a> TaskNames<'a> {
    pub fn from_tasks(raw_tasks: &'a [RawTask], default_img: Option<&'a str>) -> Result<Self> {
        let mut id_map = HashMap::new();
        let mut id = TaskId::first();
        for raw_task in raw_tasks.iter() {
            if let Some(image) = raw_task.image.as_deref().or(default_img)
                && let hash_map::Entry::Vacant(e) = id_map.entry(TaskName::Image(image))
            {
                e.insert(id.fetch_incr()?);
            }
            let task_name = TaskName::Task(&raw_task.name);
            if id_map.contains_key(&task_name) {
                return Err(Error::DuplicateTask(raw_task.name.clone()));
            }
            id_map.insert(task_name, id.fetch_incr()?);
        }
        Ok(Self(id_map))
    }

    fn get_id(&self, task_name: &TaskName) -> Option<TaskId> {
        self.0.get(task_name).cloned()
    }

    /// This should never really return [Err] as all the images are
    /// automatically added into tasks.
    pub fn get_image_id(&self, image_name: &'a str) -> Result<TaskId> {
        self.get_id(&TaskName::Image(image_name))
            .ok_or(Error::UndefinedTask(format!("pull '{image_name}'")))
    }

    pub fn get_task_id(&self, task_name: &'a str) -> Result<TaskId> {
        self.get_id(&TaskName::Task(task_name))
            .ok_or(Error::UndefinedTask(task_name.to_string()))
    }
}
