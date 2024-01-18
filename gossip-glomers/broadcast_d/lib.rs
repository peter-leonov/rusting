use anyhow::{bail, Context, Result};
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
struct Message<'a> {
    src: &'a str,
    dest: &'a str,
    body: Body<'a>,
}

fn parse_message<'a, T>(line: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    serde_json::from_str::<T>(&line).context("parsing message JSON")
}

pub fn run() -> Result<()> {
    let mut lines = io::stdin().lines();
    let mut stdout = io::stdout().lock();

    let node_id = {
        let init_line = lines
            .next()
            .context("expected a message")?
            .context("reading message")?;

        let message = parse_message::<Message>(&init_line)?;

        let Body::init(body) = message.body else {
            bail!("expected the first message to me `init`")
        };

        let outgoing = Message {
            src: body.node_id,
            dest: message.src,
            body: Body::init_ok(InitOKBody {
                typ: "init_ok",
                in_reply_to: body.msg_id,
            }),
        };

        writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
        stdout.flush()?;

        String::from(body.node_id)
    };

    let mut message_id = 0;

    for line in lines {
        let line = line.context("reading message")?;
        let message = parse_message::<Message>(&line)?;

        match message.body {
            Body::init(_) => {
                bail!("duplicate init message")
            }
            Body::init_ok(_) => {
                bail!("unexpected init_ok message")
            }
            Body::echo(body) => {
                message_id += 1;
                let outgoing = Message {
                    src: &node_id,
                    dest: message.src,
                    body: Body::echo_ok(EchoOKBody {
                        typ: "echo_ok".into(),
                        msg_id: message_id,
                        in_reply_to: body.msg_id,
                        echo: &body.echo,
                    }),
                };

                writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
                stdout.flush()?
            }
            Body::echo_ok(_) => {
                bail!("unexpected echo_ok message")
            }
        }
    }

    Ok(())
}

// {"src":"p1", "dest": "n1", "body":{"type":"init", "msg_id": 1, "node_id": "n1", "node_ids": []}}
// {"src":"n1", "dest": "n2", "body":{"type":"echo", "msg_id": 1, "echo": "foobar"}}
// {"src":"n1", "dest": "n2", "body":{"type":"echo_ok", "msg_id": 2, "in_reply_to": 1, "echo": "foobar2"}}
