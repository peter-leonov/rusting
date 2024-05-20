use anyhow::Result;
use async_channel::{self, Receiver, Sender};
use futures::channel::mpsc;
use futures::executor::block_on;
use futures::future::FusedFuture;
use futures::select;
use futures::Future;
use futures::StreamExt;
use futures::{
    future::join_all,
    future::FutureExt, // for `.fuse()`
    pin_mut,
};
use std::mem;
use std::pin::Pin;
use std::thread;
use std::time::Duration;

type Promise = Pin<Box<dyn FusedFuture<Output = Result<()>>>>;
type Closure = Box<dyn Fn(Receiver<String>) -> Promise>;

struct Dispatcher {
    promises: Vec<Promise>,
    channels: Vec<(Sender<String>, String)>,
}

async fn my_sleep(seconds: f32) {
    let (mut tx, mut rx) = mpsc::channel::<()>(1);

    thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f32(seconds));
        tx.try_send(()).unwrap();
    });

    rx.next().await;
}

impl Dispatcher {
    fn add(&mut self, cb: Closure, val: String) {
        let (tx, rx) = async_channel::bounded::<String>(10);
        self.promises.push(cb(rx));
        self.channels.push((tx, val));
    }

    async fn run(&mut self) -> Result<()> {
        let mut promises = mem::replace(&mut self.promises, Vec::new());
        let channels = mem::replace(&mut self.channels, Vec::new());

        promises.push(Box::pin(
            async move {
                for (ch, val) in channels {
                    my_sleep(1.0).await;
                    ch.send(val).await.unwrap();
                }
                Ok(())
            }
            .fuse(),
        ));

        join_all(promises).await;

        // plan
        // pass the dispatcher to all add()ed functions right away
        // in those functions await for some events from the dispatcher
        // allow add()ing more function from within functions
        // run all the futures returned from the add()ed function to completion
        // remove completed futures from time to time

        // how
        // A Listener is the functions that Dispatcher runs passing itself and then runs the futures
        // write an add() function for use in a Listener that returns a future that can be awaited within the Listener
        // pass an immutable reference to Dispatcher to Listeners and store the listeners vec in a Cell?
        // we prob don't even need the Listeners vec, we need the futures vec
        // -> looks like a wrapper for a one-off channel to me
        // -> plus an async runtime that supports dynamic task creation, fitting all in one
        // non "selected" await is not gonna work.

        // todo
        // [x] add() a few Listeners and then in the same function try to fire their awaits

        Ok(())
    }
}

async fn start() -> Result<()> {
    let mut dispatcher = Dispatcher {
        promises: Vec::new(),
        channels: Vec::new(),
    };

    dispatcher.add(
        Box::new(|rx| {
            Box::pin(
                async move {
                    dbg!(rx.recv().await.unwrap());
                    Ok(())
                }
                .fuse(),
            )
        }),
        String::from("a"),
    );

    dispatcher.add(
        Box::new(|rx| {
            Box::pin(
                async move {
                    dbg!(rx.recv().await.unwrap());
                    Ok(())
                }
                .fuse(),
            )
        }),
        String::from("b"),
    );

    dispatcher.add(
        Box::new(|rx| {
            Box::pin(
                async move {
                    dbg!(rx.recv().await.unwrap());
                    Ok(())
                }
                .fuse(),
            )
        }),
        String::from("c"),
    );

    dispatcher.run().await
}

pub fn main() -> Result<()> {
    block_on(start())
}
