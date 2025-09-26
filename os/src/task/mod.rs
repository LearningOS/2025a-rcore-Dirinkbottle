//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::mm::{MapPermission, VirtAddr, VirtPageNum};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {




    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }


}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

use crate::config::PAGE_SIZE;

///unmao some vpn
pub fn unmap_rangevpn(startaddr:VirtAddr,endaddr:VirtAddr){
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let cu = inner.current_task;
    let startvpn = VirtPageNum::from(startaddr);
    let endvpn = VirtPageNum::from(endaddr);

    //用于收集连续未映射的VPN范围
    let mut current_range_start: Option<VirtPageNum> = None;
    
    for vpn_value in (startvpn.0)..=(endvpn.0) {
        let current_vpn = VirtPageNum(vpn_value);
        
        if inner.tasks[cu].memory_set.is_vpn_mapped(current_vpn) {
            //如果当前VPN映射了，且没有正在收集的范围，则开始新范围
            if current_range_start.is_none() {
                current_range_start = Some(current_vpn);
            }
            //继续收集连续未映射的VPN
        } else {
            //遇到没映射的VPN，处理之前收集的连续映射范围
            if let Some(range_start) = current_range_start.take() {
                //取消映射从range_start到current_vpn前一个VPN的范围
                let range_end = VirtPageNum(vpn_value - 1); // 前一个VPN
                inner.tasks[cu].memory_set.unmap_range(
                    VirtAddr::from(range_start), 
                    VirtAddr(VirtAddr::from(range_end).0 + PAGE_SIZE), // 结束地址要包含整个页
                );
            }
        }
    }
    
    //处理最后一段连续未映射范围
    if let Some(range_start) = current_range_start {
        inner.tasks[cu].memory_set.unmap_range(
            VirtAddr::from(range_start), 
            VirtAddr(VirtAddr::from(endvpn).0 + PAGE_SIZE)
        );
    }

}

///map some vpn
pub fn map_rangevpn(startaddr:VirtAddr,endaddr:VirtAddr,permis:MapPermission)->isize{
  let mut inner = TASK_MANAGER.inner.exclusive_access();
    let cu = inner.current_task;
    let startvpn = VirtPageNum::from(startaddr);
    let endvpn = VirtPageNum::from(endaddr);

    //用于收集连续未映射的VPN范围
    let mut current_range_start: Option<VirtPageNum> = None;
    
    for vpn_value in (startvpn.0)..=(endvpn.0) {
        let current_vpn = VirtPageNum(vpn_value);
        
        if !inner.tasks[cu].memory_set.is_vpn_mapped(current_vpn) {
            //如果当前VPN未映射，且没有正在收集的范围，则开始新范围
            if current_range_start.is_none() {
                current_range_start = Some(current_vpn);
            }
            //继续收集连续未映射的VPN
        } else {
             //遇到已映射的VPN，处理之前收集的连续未映射范围
            if let Some(range_start) = current_range_start.take() {
                //映射从range_start到current_vpn前一个VPN的范围
                let range_end = VirtPageNum(vpn_value - 1); // 前一个VPN
                inner.tasks[cu].memory_set.insert_framed_area(
                    VirtAddr::from(range_start), 
                    VirtAddr(VirtAddr::from(range_end).0 + PAGE_SIZE), // 结束地址要包含整个页
                    permis
                );
            }
            return -1;
           
        }
    }
    
    //处理最后一段连续未映射范围
    if let Some(range_start) = current_range_start {
        inner.tasks[cu].memory_set.insert_framed_area(
            VirtAddr::from(range_start), 
            VirtAddr(VirtAddr::from(endvpn).0 + PAGE_SIZE),
            permis
        );
    }
    return 0;

}

///get syscall
pub fn getsyscalls_id(id:usize)->usize{

    let mut inner=TASK_MANAGER.inner.exclusive_access();
    let cu=inner.current_task;
    inner.tasks[cu].get_syscall_id(id)
}


///add syscall
pub fn add_syscalls_id(id:usize){
    if id > 1000{
        return;
    }
    let mut inner=TASK_MANAGER.inner.exclusive_access();
    let cu=inner.current_task;
    inner.tasks[cu].add_syscall_id(id);
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
