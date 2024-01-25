use anyhow::{Context, Result};
use flyio::{parse_message, send_message, take_init};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self};

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

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
enum BodyIn {
    #[serde(rename = "broadcast")]
    Broadcast(Broadcast),
    #[serde(rename = "read")]
    Read(Read),
    #[serde(rename = "topology")]
    Topology(Topology),
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
}

pub fn main() -> Result<()> {
    let mut lines = io::stdin().lines();
    let mut stdout = io::stdout().lock();

    let node = take_init(&mut lines, &mut stdout)?;

    let mut message_id: usize = 0;
    let mut next_message_id = move || {
        message_id += 1;
        message_id
    };

    let mut seen = Vec::<i32>::new();

    for line in lines {
        let line = line.context("reading message")?;
        // dbg!(&line);
        let message = parse_message::<BodyIn>(&line)?;

        match message.body {
            BodyIn::Broadcast(body) => {
                seen.push(body.message);

                let outgoing = BodyOut::BroadcastOK(BroadcastOK {
                    msg_id: next_message_id(),
                    in_reply_to: body.msg_id,
                });

                send_message(&mut stdout, &node.id, message.src, outgoing)?;
            }
            BodyIn::Read(body) => {
                let outgoing = BodyOut::ReadOK(ReadOK {
                    msg_id: next_message_id(),
                    in_reply_to: body.msg_id,
                    messages: &seen,
                });

                send_message(&mut stdout, &node.id, message.src, outgoing)?;
            }
            BodyIn::Topology(body) => {
                let outgoing = BodyOut::TopologyOK(TopologyOK {
                    msg_id: next_message_id(),
                    in_reply_to: body.msg_id,
                });

                send_message(&mut stdout, &node.id, message.src, outgoing)?;
            }
        }
    }

    Ok(())
}
