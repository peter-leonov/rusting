use anyhow::{anyhow, Context, Result};
use flyio::{parse_message, send_message, take_init, Message, NodeInit};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::StdoutLock;
use std::io::{self};
use std::mem;
use std::sync::mpsc::{self};
use std::sync::{atomic, Arc};
use std::thread;
use std::time::Duration;

#[derive(Deserialize, Serialize, Debug)]
struct Broadcast {
    msg_id: usize,
    message: i32,
}

#[derive(Deserialize, Serialize, Debug)]
struct BroadcastOK {
    msg_id: usize,
    in_reply_to: usize,
}

#[derive(Deserialize, Serialize, Debug)]
struct Read {
    msg_id: usize,
}

#[derive(Serialize, Debug)]
struct ReadOK<'a> {
    msg_id: usize,
    in_reply_to: usize,
    messages: &'a Vec<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Topology {
    msg_id: usize,
    topology: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
struct TopologyOK {
    msg_id: usize,
    in_reply_to: usize,
}

#[derive(Deserialize, Debug)]
struct GossipIn {
    // msg_id: usize,
    messages: Vec<i32>,
    nodes: Vec<String>,
}

#[derive(Serialize, Debug)]
struct GossipOut<'a, T> {
    msg_id: usize,
    messages: &'a [i32],
    nodes: &'a [T],
}

#[derive(Deserialize, Serialize, Debug)]
struct GossipOK {
    msg_id: usize,
    in_reply_to: usize,
}

#[derive(Deserialize, Debug)]
struct Tick;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum BodyIn {
    #[serde(rename = "broadcast")]
    Broadcast(Broadcast),
    #[serde(rename = "read")]
    Read(Read),
    #[serde(rename = "topology")]
    Topology(Topology),
    #[serde(rename = "gossip")]
    Gossip(GossipIn),
    Tick(Tick),
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
enum BodyOut<'a> {
    #[serde(rename = "broadcast_ok")]
    BroadcastOK(BroadcastOK),
    #[serde(rename = "read_ok")]
    ReadOK(ReadOK<'a>),
    #[serde(rename = "topology_ok")]
    TopologyOK(TopologyOK),
    #[serde(rename = "gossip")]
    Gossip(GossipOut<'a, String>),
}

struct Node<'a> {
    message_id: usize,
    init: NodeInit,
    my: HashSet<i32>,
    theirs: HashSet<i32>,
    lines: mpsc::Receiver<Result<String, std::io::Error>>,
    stdout: StdoutLock<'a>,
}

impl<'a> Node<'a> {
    fn next_message_id(&mut self) -> usize {
        self.message_id += 1;
        self.message_id
    }

    fn send_message<'b, T>(&mut self, dest: &'b str, body: T) -> Result<()>
    where
        T: Serialize,
    {
        send_message(&mut self.stdout, &self.init.id, dest, body).context("sending message")
    }

    fn gossip_to(&mut self, group: &[String], messages: &[i32]) -> Result<()> {
        let Some((dest, tail)) = group.split_first() else {
            return Ok(());
        };

        let msg_id = self.next_message_id();

        self.send_message(
            dest,
            BodyOut::Gossip(GossipOut {
                msg_id,
                messages,
                nodes: tail,
            }),
        )?;

        Ok(())
    }

    fn next(&mut self) -> Option<Result<Message<BodyIn>>> {
        let Ok(line) = self.lines.recv() else {
            return None;
        };

        match line {
            Err(err) => Some(Err(anyhow!(err))),
            Ok(line) => {
                if line == "tick" {
                    Some(Ok(Message {
                        src: "self".into(),
                        dest: "self".into(),
                        body: BodyIn::Tick(Tick),
                    }))
                } else {
                    Some(parse_message(&line))
                }
            }
        }
    }

    fn main(&mut self) -> Result<()> {
        loop {
            let Some(message) = self.next() else {
                break;
            };
            let Ok(message) = message else {
                if let Err(err) = message {
                    eprintln!("Application error: {err}");
                };
                continue;
            };

            match message.body {
                BodyIn::Broadcast(body) => {
                    self.my.insert(body.message);

                    let message_id = self.next_message_id();
                    self.send_message(
                        &message.src,
                        BodyOut::BroadcastOK(BroadcastOK {
                            msg_id: message_id,
                            in_reply_to: body.msg_id,
                        }),
                    )?;
                }
                BodyIn::Read(body) => {
                    let mut seen = Vec::new();
                    seen.extend(self.my.clone().into_iter());
                    seen.extend(self.theirs.clone().into_iter());
                    let outgoing = BodyOut::ReadOK(ReadOK {
                        msg_id: self.next_message_id(),
                        in_reply_to: body.msg_id,
                        messages: &seen,
                    });

                    self.send_message(&message.src, outgoing)?;
                }
                BodyIn::Topology(body) => {
                    let outgoing = BodyOut::TopologyOK(TopologyOK {
                        msg_id: self.next_message_id(),
                        in_reply_to: body.msg_id,
                    });

                    self.send_message(&message.src, outgoing)?;
                }
                BodyIn::Gossip(body) => {
                    self.theirs.extend(&body.messages);

                    let nodes = body.nodes.as_slice();
                    let (a, b) = nodes.split_at(nodes.len() / 2);
                    self.gossip_to(a, body.messages.as_slice())?;
                    self.gossip_to(b, body.messages.as_slice())?;
                }
                BodyIn::Tick(_) => {
                    // TODO
                    let node_ids = mem::take(&mut self.init.node_ids);
                    let seen_vec: Vec<_> = self.my.clone().into_iter().collect();
                    let nodes = node_ids.as_slice();
                    if nodes.len() >= 1 {
                        let (a, b) = nodes.split_at(nodes.len() / 2);
                        self.gossip_to(a, &seen_vec)?;
                        self.gossip_to(b, &seen_vec)?;
                    }
                    self.init.node_ids = node_ids;
                }
            }
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    let (send, lines) = mpsc::channel();

    let timer_send = send.clone();
    let timer_on = Arc::new(atomic::AtomicBool::new(true));
    let timer_on_clone = timer_on.clone();
    let timer = thread::spawn(move || {
        while timer_on_clone.load(atomic::Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(250));
            timer_send.send(Ok("tick".into())).unwrap();
        }
    });

    let reader = thread::spawn(move || {
        for line in io::stdin().lines() {
            send.send(line).unwrap();
        }
        timer_on.store(false, atomic::Ordering::Relaxed);
    });

    let mut stdout = io::stdout().lock();

    let node_init = take_init(&lines, &mut stdout)?;

    let mut node = Node {
        message_id: 0,
        init: node_init,
        my: HashSet::with_capacity(256),
        theirs: HashSet::with_capacity(256),
        lines,
        stdout,
    };

    node.main()?;

    timer.join().unwrap();
    // TODO: fix anyhow
    reader.join().unwrap();

    Ok(())
}
