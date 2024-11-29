use libc;
use std;
use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::os::fd::FromRawFd;

fn panic_with_last_os_error() -> ! {
    let os_error = std::io::Error::last_os_error();
    panic!("{os_error:?}")
}

unsafe fn stuff() {
    let control_fd = libc::posix_openpt(libc::O_RDWR);
    if control_fd < 0 {
        panic_with_last_os_error();
    }
    if libc::grantpt(control_fd) != 0 {
        panic_with_last_os_error();
    }

    if libc::unlockpt(control_fd) != 0 {
        panic_with_last_os_error();
    }

    let name = libc::ptsname(control_fd);
    let name = if name.is_null() {
        panic_with_last_os_error();
    } else {
        // copy the name asap
        CStr::from_ptr(name).to_owned()
    };

    dbg!(control_fd, &name);

    let pt_fd = libc::open(name.as_ptr(), libc::O_RDWR);
    if pt_fd < 0 {
        panic_with_last_os_error();
    }
    dbg!(pt_fd);

    let child_pid = libc::fork();
    if child_pid < 0 {
        panic_with_last_os_error();
    }

    if child_pid == 0 {
        println!("Starting child…");
        libc::close(control_fd);
        libc::setsid();
        if libc::dup2(pt_fd, libc::STDIN_FILENO) < 0 {
            panic_with_last_os_error();
        }
        if libc::dup2(pt_fd, libc::STDOUT_FILENO) < 0 {
            panic_with_last_os_error();
        }
        if libc::dup2(pt_fd, libc::STDERR_FILENO) < 0 {
            panic_with_last_os_error();
        }
        libc::close(pt_fd);
        let command = CString::new("ls").unwrap();
        if libc::execlp(command.as_ptr(), command.as_ptr()) < 0 {
            panic_with_last_os_error();
        };
    } else {
        dbg!(child_pid);

        std::thread::spawn(move || {
            let mut stdout = std::io::stdout();
            let mut file = std::fs::File::from_raw_fd(control_fd);
            let mut buf = [0u8; 4096];
            loop {
                let nread = file.read(&mut buf).unwrap();
                if nread == 0 {
                    break; // never happens as read() above blocks
                }
                stdout.write(&buf[0..nread]).unwrap();
            }
        });

        // let mut buf = [0u8; 1024];
        // let nread = libc::read(control_fd, buf.as_mut_ptr() as *mut libc::c_void, 1024);
        // if nread < 0 {
        //     panic_with_last_os_error();
        // }
        // libc::write(0, buf.as_mut_ptr() as *mut libc::c_void, nread as usize);

        println!("Press any key…");
        std::io::stdin().read_line(&mut String::new()).unwrap();
    }
}

fn main() {
    println!("Starting…");
    unsafe { stuff() };
    println!("Done!");
}
