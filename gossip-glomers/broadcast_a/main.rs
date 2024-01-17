use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Deserialize, Debug)]
struct InitBody {
    msg_id: usize,
    node_id: String,
    node_ids: Vec<String>,
}

#[derive(Serialize, Debug)]
struct InitOKBody {
    #[serde(rename = "type")]
    typ: &'static str,
    in_reply_to: usize,
}

#[derive(Deserialize, Debug)]
struct EchoBody {
    msg_id: usize,
    echo: String,
}

#[derive(Serialize, Debug)]
struct EchoOKBody<'a> {
    #[serde(rename = "type")]
    typ: &'static str,
    msg_id: usize,
    in_reply_to: usize,
    echo: &'a str,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum IncomingBody {
    init(InitBody),
    echo(EchoBody),
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
enum OutgingBody<'a> {
    init_ok(InitOKBody),
    echo_ok(EchoOKBody<'a>),
}

#[derive(Deserialize, Debug)]
struct IncomingMessage<T> {
    src: String,
    dest: String,
    body: T,
}

#[derive(Serialize, Debug)]
struct OutgoingMessage<T> {
    src: String,
    dest: String,
    body: T,
}

fn main() -> Result<()> {
    let mut node_id = String::new();
    let mut message_id = 0;

    let mut stdout = io::stdout().lock();

    for line in io::stdin().lines() {
        let line = line?;
        let message = serde_json::from_str::<IncomingMessage<IncomingBody>>(&line)
            .context("parsing incoming message JSON")?;

        match message.body {
            IncomingBody::init(body) => {
                node_id = body.node_id;
                message_id += 1;

                let outgoing = OutgoingMessage::<InitOKBody> {
                    src: node_id.clone(),
                    dest: message.src,
                    body: InitOKBody {
                        typ: "init_ok",
                        in_reply_to: body.msg_id,
                    },
                };

                writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
                stdout.flush()?
            }
            IncomingBody::echo(body) => {
                message_id += 1;
                let outgoing = OutgoingMessage::<EchoOKBody> {
                    src: node_id.clone(),
                    dest: message.src,
                    body: EchoOKBody {
                        typ: "echo_ok".into(),
                        msg_id: message_id,
                        in_reply_to: body.msg_id,
                        echo: &body.echo,
                    },
                };

                writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
                stdout.flush()?
            }
        }
    }

    Ok(())
}

// {"src":"n1", "dest": "n2", "body":{"type":"echo", "msg_id": 1, "echo": "foobar"}}
// {"src":"n1", "dest": "n2", "body":{"type":"echo_ok", "msg_id": 2, "in_reply_to": 1, "echo": "foobar2"}}
