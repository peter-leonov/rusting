use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Deserialize, Serialize, Debug)]
struct InitBody<'a> {
    msg_id: usize,
    node_id: &'a str,
    node_ids: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct InitOKBody<'a> {
    #[serde(rename = "type")]
    typ: &'a str,
    in_reply_to: usize,
}

#[derive(Deserialize, Serialize, Debug)]
struct EchoBody<'a> {
    msg_id: usize,
    echo: &'a str,
}

#[derive(Deserialize, Serialize, Debug)]
struct EchoOKBody<'a> {
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
    init(InitBody<'a>),
    #[serde(borrow)]
    init_ok(InitOKBody<'a>),
    #[serde(borrow)]
    echo(EchoBody<'a>),
    #[serde(borrow)]
    echo_ok(EchoOKBody<'a>),
}

#[derive(Deserialize, Serialize, Debug)]
struct Message<'a, T> {
    src: &'a str,
    dest: &'a str,
    body: T,
}

fn main() -> Result<()> {
    let mut node_id = String::new();
    let mut message_id = 0;

    let mut stdout = io::stdout().lock();

    for line in io::stdin().lines() {
        let line = line?;
        let message = serde_json::from_str::<Message<Body>>(&line)
            .context("parsing incoming message JSON")?;

        match message.body {
            Body::init(body) => {
                node_id = String::from(body.node_id);
                message_id += 1;

                let outgoing = Message::<InitOKBody> {
                    src: &node_id,
                    dest: message.src,
                    body: InitOKBody {
                        typ: "init_ok",
                        in_reply_to: body.msg_id,
                    },
                };

                writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
                stdout.flush()?
            }
            Body::init_ok(body) => {}
            Body::echo(body) => {
                message_id += 1;
                let outgoing = Message::<EchoOKBody> {
                    src: &node_id,
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
            Body::echo_ok(body) => {}
        }
    }

    Ok(())
}

// {"src":"n1", "dest": "n2", "body":{"type":"echo", "msg_id": 1, "echo": "foobar"}}
// {"src":"n1", "dest": "n2", "body":{"type":"echo_ok", "msg_id": 2, "in_reply_to": 1, "echo": "foobar2"}}
