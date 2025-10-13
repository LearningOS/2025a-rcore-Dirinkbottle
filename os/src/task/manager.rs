//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    /// 时间片长度
    pub time_slice: usize,
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}
use crate::task::task::BIGCONST;
use crate::task::TaskStatus;
/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            time_slice:5,
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
          let mut inner = task.inner_exclusive_access();
          assert_eq!(inner.task_status, TaskStatus::Ready, 
               "Only Ready tasks can be added to scheduler");
        // 关键：只有Ready状态的任务才加入队列
        if inner.task_status != TaskStatus::Ready {
            println!("[Scheduler] Task {} not Ready, status: {:?}", 
                     task.getpid(), inner.task_status);
            drop(inner);
            return;
        }
        
        // 更新调度参数
        let ticket = inner.ticket.max(1);
        inner.stride = BIGCONST / ticket;
        inner.pass = inner.pass.wrapping_add(inner.stride);
        inner.time_slice = self.time_slice;
        inner.need_resched=false;
        drop(inner);
        
        self.ready_queue.push_back(task);
    
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
          if self.ready_queue.is_empty() {
            println!("[Scheduler] Queue is empty");
            return None;
        }
  println!("[Scheduler] Queue size: {}", self.ready_queue.len());
        // 找到最小通行值的任务索引
        let mut min_index = 0;
        let mut min_pass = usize::MAX;
          
    // 打印所有任务状态
    for (_i, task) in self.ready_queue.iter().enumerate() {
        let inner = task.inner_exclusive_access();
        println!("  Task {}: status={:?}, ticket={}, pass={}", 
                 task.getpid(), inner.task_status, inner.ticket, inner.pass);
        drop(inner);
    }
        for (i, task) in self.ready_queue.iter().enumerate() {
            let inner = task.inner_exclusive_access();
            if inner.pass < min_pass {
                min_pass = inner.pass;
                min_index = i;
            }
            drop(inner);
        }

        // 移除并返回最小通行值的任务
        let task = self.ready_queue.remove(min_index).unwrap();
        
        // 更新被选中任务的通行值
        let mut inner = task.inner_exclusive_access();
        inner.pass = inner.pass.wrapping_add(inner.stride);
        drop(inner);
           println!("[Scheduler] Selected task {} with pass {}", 
             task.getpid(), min_pass);
        Some(task)
    }

/// 从队列中移除特定任务（用于退出或阻塞的任务）
    pub fn remove_task(&mut self, task: &Arc<TaskControlBlock>) {
         // 从就绪队列中移除特定任务
        if let Some(pos) = self.ready_queue.iter().position(|t| Arc::ptr_eq(t, task)) {
            self.ready_queue.remove(pos);
            println!("[Scheduler] Removed task {} from queue", task.getpid());
        }
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
