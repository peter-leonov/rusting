use anyhow::Result;
// use async_channel::{self, Receiver, Sender};
use futures::channel::oneshot;
use futures::executor::block_on;
use futures::future::FusedFuture;
// use futures::select;
// use futures::Future;
// use futures::StreamExt;
use futures::{
    future::join_all,
    future::FutureExt, // for `.fuse()`
                       // pin_mut,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::pin::Pin;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

async fn my_sleep(seconds: f32) {
    let (tx, rx) = oneshot::channel::<()>();

    thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f32(seconds));
        tx.send(()).unwrap();
    });

    rx.await.unwrap();
}

type Promise = Pin<Box<dyn FusedFuture<Output = Result<()>>>>;
type Closure = Box<dyn Fn(Rc<Inner>) -> Promise>;

struct Inner {
    promises: RefCell<Vec<Promise>>,
    channels: RefCell<HashMap<String, oneshot::Sender<String>>>,
}

impl Inner {
    fn new() -> Self {
        Self {
            promises: RefCell::new(Vec::new()),
            channels: RefCell::new(HashMap::new()),
        }
    }

    async fn listen(&self, val: String) -> String {
        let (tx, rx) = oneshot::channel::<String>();
        self.channels.borrow_mut().insert(val, tx);
        rx.await.unwrap()
    }

    fn find_channel(&self, name: &str) -> Option<oneshot::Sender<String>> {
        self.channels.borrow_mut().remove(name)
    }

    fn fire(&self, name: &str, val: String) {
        self.find_channel(name)
            .and_then(|ch| Some(ch.send(val).unwrap()));
    }
}

struct Dispatcher {
    inner: Rc<Inner>,
}

impl Dispatcher {
    fn new() -> Self {
        Self {
            inner: Rc::new(Inner::new()),
        }
    }

    fn add(&self, cb: Closure) {
        let p = cb(self.inner.clone());
        {
            let mut promises = self.inner.promises.borrow_mut();
            promises.push(p);
        };
    }

    async fn run(&self) -> Result<()> {
        let mut promises = mem::replace(self.inner.promises.borrow_mut().as_mut(), Vec::new());
        // let channels = mem::replace(&mut self.channels, RefCell::new(Vec::new()));

        promises.push(Box::pin(
            async move {
                my_sleep(0.5).await;
                self.inner.fire("a", String::from("aaa"));
                my_sleep(0.5).await;
                self.inner.fire("b", String::from("bbb"));
                Ok(())
            }
            .fuse(),
        ));

        join_all(promises).await.into_iter().collect()

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
    }
}

async fn start() -> Result<()> {
    let dispatcher = Dispatcher::new();

    dispatcher.add(Box::new(|d| {
        Box::pin(
            async move {
                dbg!(d.listen(String::from("a")).await);
                Ok(())
            }
            .fuse(),
        )
    }));

    dispatcher.add(Box::new(|d| {
        Box::pin(
            async move {
                dbg!(d.listen(String::from("b")).await);
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
