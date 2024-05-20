use anyhow::Result;
use futures::executor::block_on;
use futures::future::FusedFuture;
use futures::select;
use futures::Future;
use futures::{
    future::join_all,
    future::FutureExt, // for `.fuse()`
    pin_mut,
};
use std::mem;
use std::pin::Pin;

type Promise = Pin<Box<dyn FusedFuture<Output = Result<()>>>>;
type Closure = Box<dyn Fn(String) -> Promise>;

struct Dispatcher {
    promises: Vec<Promise>,
}

impl Dispatcher {
    fn add(&mut self, cb: Closure) {
        self.promises.push(cb("aaa".into()));
    }

    async fn run(&mut self) -> Result<()> {
        let promises = mem::replace(&mut self.promises, Vec::new());
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
        // add() a few Listeners and then in the same function try to fire their awaits

        Ok(())
    }
}

async fn start() -> Result<()> {
    let mut dispatcher = Dispatcher {
        promises: Vec::new(),
    };

    dispatcher.add(Box::new(|body| {
        Box::pin(
            async {
                dbg!(body);
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
