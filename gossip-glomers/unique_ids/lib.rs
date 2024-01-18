use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Lines, StdinLock, StdoutLock, Write};

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
#[serde(tag = "type")]
enum Body<'a> {
    #[serde(borrow)]
    init(InitBody<'a>),
    #[serde(borrow)]
    init_ok(InitOKBody<'a>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Message<'a, T> {
    pub src: &'a str,
    pub dest: &'a str,
    pub body: T,
}

pub fn parse_message<'a, T>(line: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    serde_json::from_str::<T>(&line).context("parsing message JSON")
}

pub struct InitData {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

pub fn take_init(lines: &mut Lines<StdinLock>, stdout: &mut StdoutLock) -> Result<InitData> {
    let init_line = lines
        .next()
        .context("expected a message")?
        .context("reading message")?;

    let message = parse_message::<Message<Body>>(&init_line)?;

    let Body::init(body) = message.body else {
        bail!("expected the first message to be `init`")
    };

    let outgoing = Message::<Body> {
        src: body.node_id,
        dest: message.src,
        body: Body::init_ok(InitOKBody {
            typ: "init_ok",
            in_reply_to: body.msg_id,
        }),
    };

    writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
    stdout.flush()?;

    Ok(InitData {
        node_id: String::from(body.node_id),
        node_ids: body.node_ids,
    })
}

// {"src":"p1", "dest": "n1", "body":{"type":"init", "msg_id": 1, "node_id": "n1", "node_ids": []}}
// {"src":"n1", "dest": "n2", "body":{"type":"echo", "msg_id": 1, "echo": "foobar"}}
// {"src":"n1", "dest": "n2", "body":{"type":"echo_ok", "msg_id": 2, "in_reply_to": 1, "echo": "foobar2"}}
