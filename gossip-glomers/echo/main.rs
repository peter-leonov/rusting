use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Serialize, Deserialize, Debug)]
struct InitBody {
    msg_id: usize,
    node_id: String,
    node_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InitOKBody {
    #[serde(rename = "type")]
    typ: String,
    in_reply_to: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct EchoBody {
    msg_id: usize,
    echo: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EchoOKBody {
    #[serde(rename = "type")]
    typ: String,
    msg_id: usize,
    in_reply_to: usize,
    echo: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Body {
    init(InitBody),
    init_ok(InitOKBody),
    echo(EchoBody),
    echo_ok(EchoOKBody),
}

#[derive(Serialize, Deserialize, Debug)]
struct Message<T> {
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
        let message = serde_json::from_str::<Message<Body>>(&line)
            .context("parsing incoming message JSON")?;

        match message.body {
            Body::init(body) => {
                node_id = body.node_id;
                message_id += 1;

                let outgoing = Message::<InitOKBody> {
                    src: node_id.clone(),
                    dest: message.src,
                    body: InitOKBody {
                        typ: "init_ok".into(),
                        in_reply_to: body.msg_id,
                    },
                };

                writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
                stdout.flush()?
            }
            Body::init_ok(_) => {}
            Body::echo(body) => {
                message_id += 1;
                let outgoing = Message::<EchoOKBody> {
                    src: node_id.clone(),
                    dest: message.src,
                    body: EchoOKBody {
                        typ: "echo_ok".into(),
                        msg_id: message_id,
                        in_reply_to: body.msg_id,
                        echo: body.echo,
                    },
                };

                writeln!(stdout, "{}", serde_json::to_string(&outgoing)?)?;
                stdout.flush()?
            }
            Body::echo_ok(_) => {}
        }
    }

    Ok(())
}

// {"src":"n1", "dest": "n2", "body":{"type":"echo", "msg_id": 1, "echo": "foobar"}}
// {"src":"n1", "dest": "n2", "body":{"type":"echo_ok", "msg_id": 2, "in_reply_to": 1, "echo": "foobar2"}}
