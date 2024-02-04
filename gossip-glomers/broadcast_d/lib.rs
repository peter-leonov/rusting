use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{StdoutLock, Write};
use std::sync::mpsc;

#[derive(Deserialize, Serialize, Debug)]
struct InitBody<'a> {
    msg_id: usize,
    node_id: &'a str,
    node_ids: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct InitOKBody {
    in_reply_to: usize,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
enum Body<'a> {
    #[serde(borrow, rename = "init")]
    Init(InitBody<'a>),
    #[serde(rename = "init_ok")]
    InitOK(InitOKBody),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Message<T> {
    pub src: String,
    pub dest: String,
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
    let message = Message::<T> {
        src: src.into(),
        dest: dest.into(),
        body,
    };

    serde_json::to_writer(&mut *stdout, &message)?;
    writeln!(stdout, "")?;
    Ok(stdout.flush()?)
}

pub struct NodeInit {
    pub id: String,
    pub node_ids: Vec<String>,
}

pub fn take_init(
    lines: &mut mpsc::Iter<Result<String, std::io::Error>>,
    stdout: &mut StdoutLock,
) -> Result<NodeInit> {
    let init_line = lines
        .next()
        .context("expected a message")?
        .context("reading message")?;

    let message = parse_message::<Body>(&init_line)?;

    let Body::Init(body) = message.body else {
        bail!("expected the first message to be `init`")
    };

    let node = NodeInit {
        id: String::from(body.node_id),
        node_ids: body.node_ids,
    };

    let outgoing = Body::InitOK(InitOKBody {
        in_reply_to: body.msg_id,
    });

    send_message(stdout, &node.id, &message.src, outgoing)?;

    Ok(node)
}
