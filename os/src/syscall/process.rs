//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{frame_alloc, translated_byte_buffer, PTEFlags, PageTable, VirtAddr};
use crate::task::{
    change_program_brk, current_syscall_counter, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next,
};
use crate::timer::get_time_us;

use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;

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
    // trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    // trace!("kernel: sys_get_time");
    let us = get_time_us();
    let res = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let buffer = translated_byte_buffer(
        current_user_token(),
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    unsafe {
        let kernel_bytes = core::slice::from_raw_parts(
            &res as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        );
        let mut offset = 0;
        for b in buffer {
            b.copy_from_slice(&kernel_bytes[offset..b.len()]);
            offset += b.len();
        }
    }
    0
}

/// sys_trace
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    trace!(
        "trace_request: {}, id: {}, data: {}",
        trace_request,
        id,
        data
    );
    match trace_request {
        0 => {
            let legal_virt = VirtAddr::from(id);
            // illegal address
            if id != legal_virt.0 {
                return -1;
            }
            let page = PageTable::from_token(current_user_token()).translate(legal_virt.floor());
            if page.is_none() || !page.unwrap().readable() {
                return -1;
            } else {
                let buf = translated_byte_buffer(current_user_token(), id as *const u8, 1);
                if buf.len() < 1 {
                    -1
                } else {
                    trace!("buf[0][0]: {:#?}", buf[0]);
                    buf[0][0] as isize
                }
            }
        }
        1 => {
            let legal_virt = VirtAddr::from(id);
            // illegal address
            if id != legal_virt.0 {
                return -1;
            }
            let page = PageTable::from_token(current_user_token()).translate(legal_virt.floor());
            if page.is_none() || !page.unwrap().writable() {
                return -1;
            } else {
                let ptr = id as *mut u8;
                let mut buf = translated_byte_buffer(current_user_token(), ptr, 1);
                if buf.len() < 1 {
                    -1
                } else {
                    buf[0][0] = (data & 0xFF) as u8;
                    0
                }
            }
        }
        2 => current_syscall_counter(id),
        _ => -1,
    }
}

lazy_static! {
    /// The kernel's initial memory mapping(kernel address space)
    pub static ref SYSCALL_MAP_PAGE: Arc<UPSafeCell<Vec<PageTable>>> =
        Arc::new(unsafe { UPSafeCell::new(Vec::new()) });
}

// mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    // illegal arguments
    if port & (!0x7) != 0 || port & 0x7 == 0 || start % PAGE_SIZE != 0 {
        return -1;
    }
    // ceil to page size
    let page_num = (PAGE_SIZE + len - 1) / PAGE_SIZE;
    // R W X | U
    let pte = PTEFlags::from_bits((port << 1) as u8).unwrap() | PTEFlags::U;
    let mut map_page = SYSCALL_MAP_PAGE.exclusive_access();
    let page_table = {
        let token = current_user_token();
        if let Some(i) = map_page.iter().position(|item| item.token() == token) {
            &mut map_page[i]
        } else {
            let page = PageTable::from_token(token);
            map_page.push(page);
            map_page.last_mut().unwrap()
        }
    };
    for p in 0..page_num {
        let virt = VirtAddr::from(start + p * PAGE_SIZE).floor();
        let entry = page_table.translate(virt);
        trace!(
            "map start: {:#x}, len: {}, port: {}, virt: {:#?}, entry: {:#?}",
            start,
            p,
            port,
            virt,
            entry
        );
        if entry.is_some() && entry.unwrap().is_valid() {
            return -1;
        }
        if let Some(phy_tracker) = frame_alloc() {
            trace!("mapped");
            page_table.map_with_tracker(virt, phy_tracker, pte);
        } else {
            return -1;
        }
    }

    0
}

// munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    let page_num = (PAGE_SIZE + len - 1) / PAGE_SIZE;
    let mut map_page = SYSCALL_MAP_PAGE.exclusive_access();
    let page_table = {
        let token = current_user_token();
        if let Some(i) = map_page.iter().position(|item| item.token() == token) {
            &mut map_page[i]
        } else {
            let page = PageTable::from_token(token);
            map_page.push(page);
            map_page.last_mut().unwrap()
        }
    };
    for p in 0..page_num {
        let virt = VirtAddr::from(start + p * PAGE_SIZE).floor();
        let entry = page_table.translate(virt);
        if entry.is_none() || !entry.unwrap().is_valid() {
            return -1;
        }
        trace!("unmap start: {:#x}, len: {}", start, p);
        page_table.unmap_with_tracker(virt);
    }
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
