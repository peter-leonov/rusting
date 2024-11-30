use std::io::{Read, Write};
use std::os::unix::process::CommandExt;

fn start_child() {
    println!("In the child process.");
    std::process::Command::new("./size.sh").exec();
}

fn start_parent(master: std::os::fd::OwnedFd) {
    println!("In the parent process.");

    let master = std::sync::Arc::new(std::fs::File::from(master));

    {
        let mut master = master.clone();
        std::thread::spawn(move || {
            let mut stdout = std::io::stdout();
            let mut buf = [0u8; 4096];
            loop {
                let nread = master.read(&mut buf).unwrap();
                if nread == 0 {
                    break; // never happens as read() above blocks
                }
                stdout.write(&buf[0..nread]).unwrap();
            }
        });
    }

    let address = "127.0.0.1:12345";
    let listener = std::net::TcpListener::bind(address).unwrap();
    println!("Listenning on {address}");
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        dbg!("connected", &stream);
        stream.write(b"Welcome!\n").unwrap();
        let mut master = master.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                let nread = master.read(&mut buf).unwrap();
                if nread == 0 {
                    break; // never happens as read() above blocks
                }
                if let Err(_) = stream.write(&buf[0..nread]) {
                    break;
                }
            }

            dbg!("disconnecting", &stream);
            drop(stream);
        });
    }
}

fn main() {
    println!("Starting...");
    match unsafe { nix::pty::forkpty(None, None) }.unwrap() {
        nix::pty::ForkptyResult::Parent { child: _, master } => {
            start_parent(master);
        }
        nix::pty::ForkptyResult::Child => {
            start_child();
        }
    }
}
