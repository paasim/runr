use crate::err::{Error, Result};
use std::{fmt, mem, ops};

/// A number between 0 and 255, which means 255 is also the maximum number of tasks.
/// However, this is just an implementation detail and could be changed.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u8);

impl TaskId {
    /// Return current value and get the next task id
    pub fn fetch_incr(&mut self) -> Result<Self> {
        match self.0.checked_add(1) {
            Some(v) => Ok(Self(mem::replace(&mut self.0, v))),
            None => Err(Error::TooManyTasks(self.0 as usize + 1)),
        }
    }

    /// First possible id, 0.
    pub fn first() -> Self {
        Self(0)
    }
}

impl TryFrom<usize> for TaskId {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self> {
        match value.try_into() {
            Ok(v) => Ok(Self(v)),
            Err(_) => Err(Error::TooManyTasks(value)),
        }
    }
}

impl From<TaskId> for TaskIds {
    /// [TaskIds] with only the given [TaskId] bit set.
    fn from(id: TaskId) -> Self {
        let size = id.0 & 0x7F;
        match id.0 > 0x7F {
            true => TaskIds(1 << size, 0),
            false => TaskIds(0, 1 << size),
        }
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A set of [TaskId]s.
/// Essentially a bit vector with 0 or 1 corresponding for each [TaskId].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TaskIds(u128, u128);

impl FromIterator<TaskId> for TaskIds {
    fn from_iter<T: IntoIterator<Item = TaskId>>(iter: T) -> Self {
        iter.into_iter()
            .fold(Self::default(), |ids, id| ids | TaskIds::from(id))
    }
}

impl ops::BitAnd for TaskIds {
    type Output = Self;

    /// [TaskId]s that are contained in both.
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0, self.1 & rhs.1)
    }
}

impl ops::BitAndAssign for TaskIds {
    /// [TaskId]s that are contained in both.
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
        self.1 &= rhs.1;
    }
}

impl ops::BitOr for TaskIds {
    type Output = Self;

    /// [TaskId]s that are contained in either.
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0, self.1 | rhs.1)
    }
}

impl ops::BitOrAssign for TaskIds {
    /// [TaskId]s that are contained in either.
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
        self.1 |= rhs.1;
    }
}

impl ops::Not for TaskIds {
    type Output = Self;

    /// Inverse, ie. [TaskId]s that are not included.
    fn not(self) -> Self {
        Self(!self.0, !self.1)
    }
}

impl TaskIds {
    /// Empty [TaskIds] represented by zero (again just an implementation detail).
    pub fn is_empty(&self) -> bool {
        self.1 == 0 && self.0 == 0
    }

    /// Iterate over [TaskId]s that are included.
    pub fn ids(self) -> impl Iterator<Item = TaskId> {
        (0..u8::MAX)
            .map(TaskId)
            .filter(move |i| !(self & TaskIds::from(*i)).is_empty())
    }
}

impl fmt::Display for TaskIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ids = self.ids();
        match ids.next() {
            Some(id) => write!(f, "[{id}")?,
            None => return write!(f, "[]"),
        }
        ids.try_for_each(|id| write!(f, ",{id}"))?;
        write!(f, "]")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn task_id_to_ids_is_bijective() {
        for pos in 0..255 {
            let id = TaskId::try_from(pos).unwrap();
            let ids = TaskIds::from(id);
            assert_eq!(vec![id], ids.ids().collect::<Vec<_>>())
        }
    }

    #[test]
    pub fn task_ids_add_or_not_work() {
        let ids = [TaskId::try_from(13).unwrap(), TaskId::try_from(1).unwrap()];
        let task_ids1: TaskIds = ids.into_iter().collect();
        let new_id = TaskId::try_from(99).unwrap();
        let task_ids2: TaskIds = [ids[0], ids[1], new_id].into_iter().collect();
        assert_eq!(task_ids1 | TaskIds::from(new_id), task_ids2);
        assert_eq!(task_ids1, task_ids2 & !TaskIds::from(new_id));
    }

    #[test]
    pub fn iterating_over_ids_and_collecting_is_no_op() {
        let ids = [
            TaskId::try_from(81).unwrap(),
            TaskId::try_from(13).unwrap(),
            TaskId::try_from(240).unwrap(),
            TaskId::try_from(127).unwrap(),
        ];
        let task_ids: TaskIds = ids.into_iter().collect();
        assert_eq!(task_ids.ids().count(), ids.len());
        assert_eq!(task_ids.ids().collect::<TaskIds>(), task_ids);
    }
}
