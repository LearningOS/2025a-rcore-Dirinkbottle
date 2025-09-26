//! Process management syscalls

use crate::task::{change_program_brk, current_user_token, exit_current_and_run_next, getsyscalls_id, suspend_current_and_run_next, unmap_rangevpn};
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?

use crate::mm::{ MapPermission, PageTable, VirtAddr,  translated_byte_buffer};
use crate::timer::get_time_us;
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
   let ptr= translated_byte_buffer(current_user_token(), _ts as *const u8, 16).pop().unwrap().as_mut_ptr() as *mut TimeVal ;
    unsafe {
        *ptr=TimeVal { sec: get_time_us()/1_000_000, usec: get_time_us()%1_000_000 }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {

    match _trace_request{
        0=>{
           let table= PageTable::from_token(current_user_token());
           match table.translate(VirtAddr(_id).floor()){
            None=>{
               return  -1;
            }
            Some(_value)=>{
             if _value.is_valid() && _value.readable(){
                 let ppn=_value.ppn().0;
                 let offset = _id % PAGE_SIZE;
                 let paddr=(ppn*PAGE_SIZE + offset )as *mut u8;
                unsafe {
                   return  (*paddr) as isize;
                }
                
             }else {
                 return -1;
             }
            }
           }
        }
        1=>{
            let table= PageTable::from_token(current_user_token());
           match table.translate(VirtAddr(_id).floor()){
            None=>{
               return  -1;
            }
            Some(_value)=>{
               if !_value.writable(){return -1;}
               if !_value.is_valid(){return -1;}
                let ppn=_value.ppn().0;
                 let offset = _id % PAGE_SIZE;
                 let paddr=(ppn*PAGE_SIZE + offset )as *mut u8;
                unsafe {
                    *paddr =_data as u8;
                    return 0;
                }
            
            

            }
           }

           
        }
        2=>{
            getsyscalls_id(_id) as isize
        }
        _=>{-1}
    }
}
use crate::task::map_rangevpn;
use crate::config::PAGE_SIZE;
// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {

    //权限位检查
     //0 0 0 0 0 0 0 0 
    if _port ==0 || _port>=8{
        return -1;
    }

    //地址没对齐 或者映射大小没对齐
     if _start%PAGE_SIZE!=0 {
        return -1;
    }

  //  let aligned_start = VirtAddr(_start).floor();
    //let aligned_end = VirtAddr(_start+_len).ceil();
    let mut aligin_len:usize=0;
    if _len%PAGE_SIZE!=0{
        aligin_len=(_len/PAGE_SIZE+1)*PAGE_SIZE;
    }

    let startaddr=VirtAddr::from(_start);
    let endaddr=VirtAddr::from(_start+aligin_len);
    let mut newper=MapPermission::empty();
    let perm=MapPermission::from_bits_truncate(_port as u8);
    newper |=perm;
    return map_rangevpn(startaddr, endaddr, newper);
    
    
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    
    //地址没对齐 或者映射大小没对齐
     if _start%PAGE_SIZE!=0 || _len%PAGE_SIZE!=0{
        return -1;
    }

  //  let aligned_start = VirtAddr(_start).floor();
    //let aligned_end = VirtAddr(_start+_len).ceil();


    let startaddr=VirtAddr::from(_start);
    let endaddr=VirtAddr::from(_start+_len);
    unmap_rangevpn(startaddr, endaddr);
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
