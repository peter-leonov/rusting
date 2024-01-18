use anyhow::{bail, Context, Result};
use flyio::{parse_message, send_message, take_init};
use serde::{Deserialize, Serialize};
use std::io::{self};

#[derive(Deserialize, Serialize, Debug)]
struct Echo<'a> {
    msg_id: usize,
    echo: &'a str,
}

#[derive(Deserialize, Serialize, Debug)]
struct EchoOK<'a> {
    #[serde(rename = "type")]
    typ: &'a str,
    msg_id: usize,
    in_reply_to: usize,
    echo: &'a str,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
enum Body<'a> {
    #[serde(borrow)]
    echo(Echo<'a>),
    #[serde(borrow)]
    echo_ok(EchoOK<'a>),
}

pub fn main() -> Result<()> {
    let mut lines = io::stdin().lines();
    let mut stdout = io::stdout().lock();

    let init = take_init(&mut lines, &mut stdout)?;
    let node_id = init.node_id;

    let mut message_id = 0;

    for line in lines {
        let line = line.context("reading message")?;
        let message = parse_message::<Body>(&line)?;

        match message.body {
            Body::echo(body) => {
                message_id += 1;
                let outgoing = Body::echo_ok(EchoOK {
                    typ: "echo_ok".into(),
                    msg_id: message_id,
                    in_reply_to: body.msg_id,
                    echo: &body.echo,
                });

                send_message(&mut stdout, &node_id, message.src, outgoing)?;
            }
            Body::echo_ok(_) => {
                bail!("unexpected echo_ok message")
            }
        }
    }

    Ok(())
}
