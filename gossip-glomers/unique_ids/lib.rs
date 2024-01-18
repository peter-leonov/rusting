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
    #[serde(borrow, rename = "init")]
    Init(InitBody<'a>),
    #[serde(borrow, rename = "init_ok")]
    InitOK(InitOKBody<'a>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Message<'a, T> {
    pub src: &'a str,
    pub dest: &'a str,
    pub body: T,
}

pub fn parse_message<'a, T>(line: &'a str) -> Result<Message<T>>
where
    T: Deserialize<'a>,
{
    serde_json::from_str::<Message<T>>(&line).context("parsing message JSON")
}

pub fn send_message<'a, T>(
    stdout: &mut StdoutLock,
    src: &'a str,
    dest: &'a str,
    body: T,
) -> Result<()>
where
    T: Serialize,
{
    let message = Message::<T> { src, dest, body };

    writeln!(stdout, "{}", serde_json::to_string(&message)?)?;
    Ok(stdout.flush()?)
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

    let message = parse_message::<Body>(&init_line)?;

    let Body::Init(body) = message.body else {
        bail!("expected the first message to be `init`")
    };

    let outgoing = Body::InitOK(InitOKBody {
        typ: "init_ok",
        in_reply_to: body.msg_id,
    });

    send_message(stdout, &body.node_id, message.src, outgoing)?;

    Ok(InitData {
        node_id: String::from(body.node_id),
        node_ids: body.node_ids,
    })
}
