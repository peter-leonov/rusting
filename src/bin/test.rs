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
use std::cell::RefCell;
use std::mem;
use std::pin::Pin;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

type Promise<'a> = Pin<Box<dyn FusedFuture<Output = Result<()>> + 'a>>;
type Closure<'a> = Box<dyn Fn(&'a Dispatcher) -> Promise<'a>>;

struct Dispatcher<'a> {
    promises: RefCell<Vec<Promise<'a>>>,
    channels: RefCell<Vec<(Sender<String>, String)>>,
}

async fn my_sleep(seconds: f32) {
    let (mut tx, mut rx) = mpsc::channel::<()>(1);

    thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f32(seconds));
        tx.try_send(()).unwrap();
    });

    rx.next().await;
}

impl<'a> Dispatcher<'a> {
    fn add(&'a self, cb: Closure<'a>) {
        let (tx, rx) = async_channel::bounded::<String>(10);
        let p = cb(self);
        {
            let mut promises = self.promises.borrow_mut();
            promises.push(p);
        };
    }

    fn listen(&self, val: String) -> Promise {
        let (tx, rx) = async_channel::bounded::<String>(1);
        {
            let channels = self.channels.borrow_mut();
            channels.push((tx, val));
        };

        Box::pin(
            async move {
                rx.recv().await?;
                Ok(())
            }
            .fuse(),
        )
        // when we .await in the caller, the join_all() below is still blocked
        // and it does not know about the new channel created here, thus there is
        // nobody to send(), thus a deadlock is created. channels must be the multiplexer.
    }

    async fn run(&self) -> Result<()> {
        let mut promises = {
            let mut promises = self.promises.borrow_mut();
            mem::replace(promises.as_mut(), Vec::new())
        };
        // let channels = mem::replace(&mut self.channels, RefCell::new(Vec::new()));

        promises.push(Box::pin(
            async move {
                // Cannot borrow and iterate as the cell is shared
                // within the listeners.
                // loop {}

                // for (ch, val) in channels {
                //     my_sleep(1.0).await;
                //     ch.send(val).await.unwrap();
                // }
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
    let dispatcher = Dispatcher {
        promises: RefCell::new(Vec::new()),
        channels: RefCell::new(Vec::new()),
    };

    dispatcher.add(Box::new(|d: &Dispatcher| {
        Box::pin(
            async {
                dbg!(d.listen(String::from("a")).await.unwrap());
                Ok(())
            }
            .fuse(),
        )
    }));

    dispatcher.run().await
}

pub fn main() -> Result<()> {
    block_on(start())
}
