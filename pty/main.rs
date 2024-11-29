use libc;
use nix;
use std;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;

fn main() {
    match unsafe { nix::pty::forkpty(None, None) }.unwrap() {
        nix::pty::ForkptyResult::Parent { child: _, master } => {
            println!("In the parent process");
            std::thread::spawn(move || {
                let mut stdout = std::io::stdout();
                let mut file = std::fs::File::from(master);
                let mut buf = [0u8; 4096];
                loop {
                    let nread = file.read(&mut buf).unwrap();
                    if nread == 0 {
                        break; // never happens as read() above blocks
                    }
                    stdout.write(&buf[0..nread]).unwrap();
                }
            });

            println!("Press Enter to exit parent...");
            std::io::stdin().read_line(&mut String::new()).unwrap();
        }
        nix::pty::ForkptyResult::Child => {
            println!("In the child process");
            unsafe { libc::setsid() };
            std::process::Command::new("ls").exec();
        }
    }
}
