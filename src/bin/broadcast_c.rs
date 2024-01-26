use anyhow::{Context, Result};
use flyio::{parse_message, send_message, take_init, NodeInit};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self};
use std::io::{Lines, StdinLock, StdoutLock, Write};

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
struct GossipIn<'a> {
    msg_id: usize,
    messages: Vec<i32>,
    #[serde(borrow)]
    nodes: Vec<&'a str>,
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
enum BodyIn<'a> {
    #[serde(rename = "broadcast")]
    Broadcast(Broadcast),
    #[serde(rename = "read")]
    Read(Read),
    #[serde(rename = "topology")]
    Topology(Topology),
    #[serde(rename = "gossip")]
    #[serde(borrow)]
    Gossip(GossipIn<'a>),
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
    #[serde(rename = "gossip")]
    GossipRef(GossipOut<'a, &'a str>),
    #[serde(rename = "gossip_ok")]
    GossipOK(GossipOK),
}

struct Node {
    message_id: usize,
    init: NodeInit,
}

impl Node {
    fn next_message_id(&mut self) -> usize {
        self.message_id += 1;
        self.message_id
    }

    fn main(&mut self, lines: &mut Lines<StdinLock>, mut stdout: &mut StdoutLock) -> Result<()> {
        let mut seen = Vec::<i32>::with_capacity(100);

        for line in lines {
            let line = line.context("reading message")?;
            // dbg!(&line);
            let message = parse_message::<BodyIn>(&line)?;

            match message.body {
                BodyIn::Broadcast(body) => {
                    seen.push(body.message);

                    let message_id = self.next_message_id();
                    send_message(
                        stdout,
                        &self.init.id,
                        message.src,
                        BodyOut::BroadcastOK(BroadcastOK {
                            msg_id: message_id,
                            in_reply_to: body.msg_id,
                        }),
                    )?;

                    let message_id = self.next_message_id();
                    let mut gossip_to = |group: &[String]| -> Result<()> {
                        if let Some((head, tail)) = group.split_first() {
                            send_message(
                                stdout,
                                &self.init.id,
                                head,
                                BodyOut::Gossip(GossipOut {
                                    msg_id: message_id,
                                    messages: &[body.message],
                                    nodes: tail,
                                }),
                            )?;
                        }

                        Ok(())
                    };

                    let nodes = self.init.node_ids.as_slice();
                    if nodes.len() >= 1 {
                        let (a, b) = nodes.split_at(nodes.len() / 2);
                        gossip_to(a)?;
                        gossip_to(b)?;
                    }
                }
                BodyIn::Read(body) => {
                    let outgoing = BodyOut::ReadOK(ReadOK {
                        msg_id: self.next_message_id(),
                        in_reply_to: body.msg_id,
                        messages: &seen,
                    });

                    send_message(&mut stdout, &self.init.id, message.src, outgoing)?;
                }
                BodyIn::Topology(body) => {
                    let outgoing = BodyOut::TopologyOK(TopologyOK {
                        msg_id: self.next_message_id(),
                        in_reply_to: body.msg_id,
                    });

                    send_message(&mut stdout, &self.init.id, message.src, outgoing)?;
                }
                BodyIn::Gossip(body) => {
                    seen.extend_from_slice(body.messages.as_slice());

                    let message_id = self.next_message_id();
                    send_message(
                        stdout,
                        &self.init.id,
                        message.src,
                        BodyOut::GossipOK(GossipOK {
                            msg_id: message_id,
                            in_reply_to: body.msg_id,
                        }),
                    )?;

                    let mut gossip_to = |group: &[&str]| -> Result<()> {
                        if let Some((head, tail)) = group.split_first() {
                            let message_id = self.next_message_id();
                            send_message(
                                stdout,
                                &self.init.id,
                                head,
                                BodyOut::GossipRef(GossipOut {
                                    msg_id: message_id,
                                    messages: body.messages.as_slice(),
                                    nodes: tail,
                                }),
                            )?;
                        }

                        Ok(())
                    };

                    let nodes = body.nodes.as_slice();
                    let (a, b) = nodes.split_at(nodes.len() / 2);
                    gossip_to(a)?;
                    gossip_to(b)?;
                }
                BodyIn::GossipOK(_) => {}
            }
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    let mut lines = io::stdin().lines();
    let mut stdout = io::stdout().lock();

    let node_init = take_init(&mut lines, &mut stdout)?;

    let mut node = Node {
        message_id: 0,
        init: node_init,
    };

    node.main(&mut lines, &mut stdout)
}
