//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{
    block_current_and_run_next, current_process, current_task, wakeup_task, TaskControlBlock,
};
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self, sem_id: usize) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                {
                    let task_inner = task.inner_exclusive_access();
                    let tid = task_inner.res.as_ref().unwrap().tid;
                    let proc_out = task_inner.res.as_ref().unwrap().process.upgrade().unwrap();
                    let mut proc = proc_out.inner_exclusive_access();
                    if proc.dead_detect {
                        let pos = proc
                            .blocking_task
                            .iter()
                            .position(|(thread_id, semaphore_id)| {
                                if *thread_id == tid && semaphore_id.unwrap() == sem_id {
                                    true
                                } else {
                                    false
                                }
                            })
                            .unwrap();
                        proc.blocking_task.remove(pos);
                    }
                }
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);

            dead_semaphore_detect();

            block_current_and_run_next();
        }
    }

    /// available count of semaphore
    pub fn available(&self) -> isize {
        let inner = self.inner.exclusive_access();
        inner.count
    }

    /// up operation from dead semaphore
    pub fn up_from_dead(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.wait_queue.pop_back();
        inner.count += 1;
    }
}

/// detect dead semaphore and remove it
pub fn dead_semaphore_detect() {
    let proc_out = current_process();
    let mut proc = proc_out.inner_exclusive_access();
    if !proc.dead_detect {
        return;
    }
    let mut task_tot = 0;
    proc.tasks.iter().for_each(|t_opt| {
        if let Some(t) = t_opt {
            let t_inner = t.inner_exclusive_access();
            if t_inner.exit_code.is_none() {
                // debug!("task no available, {}", t_inner.res.as_ref().unwrap().tid);
                task_tot += 1;
            }
        }
    });
    debug!(
        "blocking_task: {:?}, task_total: {}",
        proc.blocking_task, task_tot
    );
    if proc.blocking_task.len() >= task_tot {
        // dead occur

        // find the last blocking semaphore
        let (pos, tid, sem_id) = proc
            .blocking_task
            .iter()
            .enumerate()
            .rev()
            .find_map(|(pos, (tid, sem_opt))| {
                if let Some(sem_id) = sem_opt {
                    Some((pos, tid, sem_id))
                } else {
                    None
                }
            })
            .unwrap();
        let task = proc.tasks[*tid].as_ref().unwrap().clone();
        task.inner_exclusive_access().dead_semaphore = true;

        proc.semaphore_list[*sem_id]
            .as_ref()
            .unwrap()
            .up_from_dead();
        proc.blocking_task.remove(pos);
        wakeup_task(task);
    }
}
