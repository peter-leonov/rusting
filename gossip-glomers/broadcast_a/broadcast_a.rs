use anyhow::{bail, Context, Result};
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

#[derive(Deserialize, Serialize, Debug)]
struct ReadOK {
    msg_id: usize,
    in_reply_to: usize,
    messages: Vec<i32>,
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
enum Body {
    #[serde(rename = "broadcast")]
    Broadcast(Broadcast),
    #[serde(rename = "broadcast_ok")]
    BroadcastOK(BroadcastOK),
    #[serde(rename = "read")]
    Read(Read),
    #[serde(rename = "read_ok")]
    ReadOK(ReadOK),
    #[serde(rename = "topology")]
    Topology(Topology),
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
        let message = parse_message::<Body>(&line)?;

        match message.body {
            Body::Broadcast(body) => {
                seen.push(body.message);

                let outgoing = Body::BroadcastOK(BroadcastOK {
                    msg_id: next_message_id(),
                    in_reply_to: body.msg_id,
                });

                send_message(&mut stdout, &node.id, message.src, outgoing)?;
            }
            Body::BroadcastOK(_) => {
                bail!("unexpected broadcast_ok message")
            }
            Body::Read(body) => {
                let outgoing = Body::ReadOK(ReadOK {
                    msg_id: next_message_id(),
                    in_reply_to: body.msg_id,
                    messages: seen.clone(),
                });

                send_message(&mut stdout, &node.id, message.src, outgoing)?;
            }
            Body::ReadOK(_) => bail!("unexpected read_ok message"),
            Body::Topology(body) => {
                let outgoing = Body::TopologyOK(TopologyOK {
                    msg_id: next_message_id(),
                    in_reply_to: body.msg_id,
                });

                send_message(&mut stdout, &node.id, message.src, outgoing)?;
            }
            Body::TopologyOK(_) => bail!("unexpected topology_ok message"),
        }
    }

    Ok(())
}
