//! Process management syscalls
use core::{ptr::null};

use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next,get_syscallss_id},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    unsafe {
        *_ts = TimeVal {
            sec: get_time_us() / 1_000_000,
            usec: get_time_us() % 1_000_000,
        };
    }
    0
}


// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    match _trace_request{
        0=>{
            if _id as *const u8 == null() || _id ==0 {
                return -1;
            }

            let ptr = _id as *const u8;
            unsafe {
                *ptr  as isize
            }
            



        }
        1=>{
            if _id as *const u8 == null() || _id ==0 {
                return -1;
            }
            let  ptr = _id as *mut u8;
            unsafe {
                *ptr=_data as u8;
            }
            0

        }
        2=>{
            get_syscallss_id(_id) as isize
        }
        _=>{
            -1
        }
    }
}
