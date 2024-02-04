use anyhow::{anyhow, bail, Context, Result};
use flyio::{parse_message, send_message, take_init, Message, NodeInit};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::StdoutLock;
use std::io::{self};
use std::mem;
use std::sync::mpsc::{self, RecvTimeoutError};
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
    msg_id: usize,
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
    #[serde(rename = "gossip_ok")]
    GossipOK(GossipOK),
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
    #[serde(rename = "gossip_ok")]
    GossipOK(GossipOK),
}

struct Node<'a> {
    message_buffer: Vec<Message<BodyIn>>,
    message_id: usize,
    init: NodeInit,
    seen: Vec<i32>,
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

    fn wait_for(&mut self, src: &str, in_reply_to: usize) -> Option<Result<Message<BodyIn>>> {
        loop {
            let Some(next) = self.next_timeout() else {
                return None;
            };

            if let Some(res) = next {
                let Ok(message) = res else {
                    return Some(res);
                };

                if message.src != src {
                    self.message_buffer.push(message);
                    continue;
                }

                match message.body {
                    BodyIn::GossipOK(ref body) => {
                        if body.in_reply_to == in_reply_to {
                            return Some(Ok(message));
                        }
                    }
                    _ => {
                        self.message_buffer.push(message);
                    }
                }
            } else {
                return Some(Err(anyhow!(
                    "stdin EOF while waiting for a message from {} in reply to {}",
                    src,
                    in_reply_to
                )));
            }
        }
    }

    fn gossip_to(&mut self, group: &[String], messages: &[i32]) -> Result<()> {
        let Some((dest, tail)) = group.split_first() else {
            return Ok(());
        };

        let msg_id = self.next_message_id();

        loop {
            self.send_message(
                dest,
                BodyOut::Gossip(GossipOut {
                    msg_id,
                    messages,
                    nodes: tail,
                }),
            )?;

            let Some(ok) = self.wait_for(dest, msg_id) else {
                continue;
            };

            ok?;

            break;
        }

        Ok(())
    }

    fn next(&mut self) -> Option<Result<Message<BodyIn>>> {
        let Ok(line) = self.lines.recv() else {
            return None;
        };

        match line {
            Err(err) => Some(Err(anyhow!(err))),
            Ok(line) => Some(parse_message(&line)),
        }
    }

    fn next_timeout(&mut self) -> Option<Option<Result<Message<BodyIn>>>> {
        match self.lines.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => match line {
                Err(err) => Some(Some(Err(anyhow!(err)))),
                Ok(line) => Some(Some(parse_message(&line))),
            },
            Err(err) => match err {
                RecvTimeoutError::Timeout => None,
                RecvTimeoutError::Disconnected => Some(None),
            },
        }
    }

    fn main(&mut self) -> Result<()> {
        loop {
            let Some(message) = self.next() else { break };
            let message = message?;

            match message.body {
                BodyIn::Broadcast(body) => {
                    self.seen.push(body.message);

                    let message_id = self.next_message_id();
                    self.send_message(
                        &message.src,
                        BodyOut::BroadcastOK(BroadcastOK {
                            msg_id: message_id,
                            in_reply_to: body.msg_id,
                        }),
                    )?;

                    // TODO
                    let node_ids = mem::take(&mut self.init.node_ids);
                    let nodes = node_ids.as_slice();
                    if nodes.len() >= 1 {
                        let (a, b) = nodes.split_at(nodes.len() / 2);
                        self.gossip_to(a, &[body.message])?;
                        self.gossip_to(b, &[body.message])?;
                    }
                    self.init.node_ids = node_ids
                }
                BodyIn::Read(body) => {
                    // TODO
                    let seen = mem::take(&mut self.seen);
                    let outgoing = BodyOut::ReadOK(ReadOK {
                        msg_id: self.next_message_id(),
                        in_reply_to: body.msg_id,
                        messages: &seen,
                    });

                    self.send_message(&message.src, outgoing)?;
                    self.seen = seen;
                }
                BodyIn::Topology(body) => {
                    let outgoing = BodyOut::TopologyOK(TopologyOK {
                        msg_id: self.next_message_id(),
                        in_reply_to: body.msg_id,
                    });

                    self.send_message(&message.src, outgoing)?;
                }
                BodyIn::Gossip(body) => {
                    self.seen.extend_from_slice(body.messages.as_slice());

                    let message_id = self.next_message_id();
                    self.send_message(
                        &message.src,
                        BodyOut::GossipOK(GossipOK {
                            msg_id: message_id,
                            in_reply_to: body.msg_id,
                        }),
                    )?;

                    let nodes = body.nodes.as_slice();
                    let (a, b) = nodes.split_at(nodes.len() / 2);
                    self.gossip_to(a, body.messages.as_slice())?;
                    self.gossip_to(b, body.messages.as_slice())?;
                }
                BodyIn::GossipOK(_) => {
                    self.message_buffer.push(message);
                }
            }
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    let (send, lines) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in io::stdin().lines() {
            send.send(line).unwrap();
        }
    });

    let mut stdout = io::stdout().lock();

    let node_init = take_init(&lines, &mut stdout)?;

    let mut node = Node {
        message_buffer: Vec::with_capacity(100),
        message_id: 0,
        init: node_init,
        seen: Vec::<i32>::with_capacity(100),
        lines,
        stdout,
    };

    node.main()?;

    // TODO: fix anyhow
    reader.join().unwrap();

    Ok(())
}
