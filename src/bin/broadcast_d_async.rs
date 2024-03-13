use anyhow::{anyhow, Context, Result};
use futures::{channel::mpsc, channel::mpsc::Receiver, executor::block_on};
use futures::{Future, StreamExt};
// use futures::Future;
use flyio::{parse_message, send_message, take_init, take_init_line, Message, NodeInit};
// use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
// use std::io::StdoutLock;
use std::io;
use std::pin::Pin;
// use std::mem;
// use std::sync::mpsc::{self};
// use std::sync::{atomic, Arc};
use std::thread;
use std::time::Duration;

#[derive(Deserialize, Serialize, Debug, Clone)]
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

/// Super inefficient async sleep
async fn my_sleep(delay: f32) {
    let (mut tx, mut rx) = mpsc::channel::<()>(1);

    thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f32(delay));
        tx.try_send(()).unwrap();
    });

    rx.next().await;
}

async fn process(mut rx: Receiver<String>) -> Result<()> {
    let init_line = rx.next().await.unwrap();

    let node_init = take_init_line(&init_line)?;

    let mut broadcast_listeners: Vec<
        Box<dyn Fn(Broadcast) -> Pin<Box<dyn Future<Output = Result<bool>>>>>,
    > = Vec::with_capacity(100);

    broadcast_listeners.push(Box::new(|body| {
        Box::pin(async {
            dbg!("first");
            dbg!(body);
            Ok(false)
        })
    }));

    while let Some(line) = rx.next().await {
        // my_sleep(0.250).await;

        let message = parse_message::<BodyIn>(&line)?;
        match message.body {
            BodyIn::Broadcast(body) => {
                // dbg!(body);
                let mut add_back = Vec::with_capacity(100);
                for f in broadcast_listeners.drain(..) {
                    if !f(body.clone()).await? {
                        add_back.push(f);
                    }
                }
                broadcast_listeners = add_back;

                broadcast_listeners.push(Box::new(|body| {
                    Box::pin(async {
                        dbg!("second");
                        dbg!(body);
                        Ok(true)
                    })
                }));
            }
            _ => {
                todo!();
            }
        }
    }

    rx.close();
    Ok(())
}

pub fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<String>(10);

    let lines = thread::spawn(move || -> Result<()> {
        let mut tx = tx;
        for line in io::stdin().lines() {
            tx.try_send(line?)?;
        }

        Ok(())
    });

    block_on(process(rx))?;
    lines.join().unwrap()?;

    Ok(())
}
