use anyhow::{anyhow, Context, Result};
use async_channel::{self, Receiver, Sender};
use futures::{channel::mpsc, executor::block_on};
use futures::{Future, SinkExt, StreamExt};
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

struct Listener {
    broadcast: Vec<Box<dyn Fn(Broadcast) -> Pin<Box<dyn Future<Output = Result<bool>>>>>>,
}

impl Listener {
    fn new() -> Self {
        Listener {
            broadcast: Vec::new(),
        }
    }

    async fn process(&mut self, mut rx: Receiver<String>) -> Result<()> {
        self.broadcast.push(Box::new(|body| {
            Box::pin(async {
                dbg!("first");
                dbg!(body);
                Ok(false)
            })
        }));

        while let Ok(line) = rx.recv().await {
            // my_sleep(0.250).await;

            let message = parse_message::<BodyIn>(&line)?;
            dbg!(&message);
            match message.body {
                BodyIn::Broadcast(body) => {
                    // dbg!(body);
                    let mut add_back = Vec::with_capacity(100);
                    for f in self.broadcast.drain(..) {
                        if !f(body.clone()).await? {
                            add_back.push(f);
                        }
                    }
                    self.broadcast = add_back;

                    self.broadcast.push(Box::new(|body| {
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
}

async fn start(rx: Receiver<String>, tx: Sender<String>) -> Result<()> {
    tx.send("asdfsd".into()).await?;

    let init_line = rx.recv().await?;

    let node_init = take_init_line(&init_line)?;
    dbg!(node_init);

    Listener::new().process(rx).await?;

    Ok(())
}

pub fn main() -> Result<()> {
    let (input_tx, input_rx) = async_channel::bounded::<String>(10);
    let input_thread = thread::spawn(move || -> Result<()> {
        let tx = input_tx;
        for line in io::stdin().lines() {
            tx.try_send(line?)?;
        }

        Ok(())
    });

    let (output_tx, output_rx) = async_channel::bounded::<String>(10);
    let output_thread = thread::spawn(move || -> Result<()> {
        let rx = output_rx;
        while let Ok(line) = rx.recv_blocking() {
            dbg!(line);
        }
        dbg!("!!!!!!!!!!!!");
        Ok(())
    });

    block_on(start(input_rx, output_tx))?;
    input_thread.join().unwrap()?;
    output_thread.join().unwrap()?;

    Ok(())
}
