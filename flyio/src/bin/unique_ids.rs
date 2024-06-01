use anyhow::{bail, Context, Result};
use flyio::{parse_message, send_message, take_init};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::io::{self};

#[derive(Deserialize, Serialize, Debug)]
struct Generate {
    msg_id: usize,
}

#[derive(Deserialize, Serialize, Debug)]
struct GenerateOK<'a> {
    msg_id: usize,
    in_reply_to: usize,
    id: &'a str,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
enum Body<'a> {
    #[serde(rename = "generate")]
    Generate(Generate),
    #[serde(borrow, rename = "generate_ok")]
    GenerateOK(GenerateOK<'a>),
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

    let mut id = String::with_capacity(100);

    for line in lines {
        let line = line.context("reading message")?;
        let message = parse_message::<Body>(&line)?;

        match message.body {
            Body::Generate(body) => {
                let msg_id = next_message_id();

                id.clear();
                write!(id, "{}-{}", &node.id, msg_id)?;

                let outgoing = Body::GenerateOK(GenerateOK {
                    msg_id,
                    in_reply_to: body.msg_id,
                    id: &id,
                });

                send_message(&mut stdout, &node.id, message.src, outgoing)?;
            }
            Body::GenerateOK(_) => {
                bail!("unexpected generate_ok message")
            }
        }
    }

    Ok(())
}
