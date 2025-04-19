const SBI_SHUTDOWN: usize = 8;

fn sbi_call(which: usize, args: [usize; 3]) -> usize {
    let mut ret;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") which,
           
        )
    }
    ret
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, [0, 0, 0]);
    panic!("It should shutdown!");
}
