use std::io::{Read, Write};
use std::os::unix::process::CommandExt;

fn start_child() {
    println!("In the child process.");
    std::process::Command::new("bash").exec();
}

fn start_parent(stream: std::net::TcpStream, master: std::os::fd::OwnedFd) {
    println!("In the parent process.");

    let master = std::sync::Arc::new(std::fs::File::from(master));

    let mut stream1 = stream.try_clone().unwrap();
    let mut stream2 = stream.try_clone().unwrap();

    {
        let mut master = master.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                let nread = master.read(&mut buf).unwrap();
                dbg!("master.read", nread);
                if nread == 0 {
                    break;
                }

                if let Err(_) = stream1.write(&buf[0..nread]) {
                    break;
                }
            }
            println!("master.read done");
        });
    }

    {
        let mut master = master.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                if let Ok(nread) = stream2.read(&mut buf) {
                    dbg!("stream2.read", nread);
                    master.write(&buf[0..nread]).unwrap();
                    if nread == 0 {
                        break; // never happens as read() above blocks
                    }
                } else {
                    break;
                }
            }
        });
    }

    dbg!("Done!");
}

fn handle_connection(stream: std::net::TcpStream) {
    dbg!("handle", &stream);
    writeln!(&stream, "Hi").expect("write to succeed");
    match unsafe { nix::pty::forkpty(None, None) }.unwrap() {
        nix::pty::ForkptyResult::Parent { child: _, master } => {
            start_parent(stream, master);
        }
        nix::pty::ForkptyResult::Child => {
            start_child();
        }
    }
}

fn main() {
    println!("Starting...");

    let address = "127.0.0.1:12345";
    let listener = std::net::TcpListener::bind(address).unwrap();
    println!("Listenning on {address}");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("connected");
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            eprintln!("can't fork");
        } else if pid == 0 {
            handle_connection(stream);
        }
    }
}
