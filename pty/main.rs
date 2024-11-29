use libc;
use std;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;

fn panic_with_last_os_error() -> ! {
    let os_error = std::io::Error::last_os_error();
    panic!("{os_error:?}")
}

unsafe fn stuff() {
    let mut amaster = -1i32;
    let child_pid = libc::forkpty(
        &mut amaster as *mut i32,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if child_pid < 0 {
        panic_with_last_os_error();
    }

    if child_pid == 0 {
        println!("Starting child…");
        libc::setsid();
        std::process::Command::new("ls").exec();
    } else {
        if amaster < 0 {
            panic_with_last_os_error();
        }

        std::thread::spawn(move || {
            let mut stdout = std::io::stdout();
            let mut file = std::fs::File::from_raw_fd(amaster);
            let mut buf = [0u8; 4096];
            loop {
                let nread = file.read(&mut buf).unwrap();
                if nread == 0 {
                    break; // never happens as read() above blocks
                }
                stdout.write(&buf[0..nread]).unwrap();
            }
        });

        println!("Press Enter to continue...");
        std::io::stdin().read_line(&mut String::new()).unwrap();
    }
}

fn main() {
    println!("Starting…");
    unsafe { stuff() };
    println!("Done!");
}
