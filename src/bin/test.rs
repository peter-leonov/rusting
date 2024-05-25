use anyhow::Result;
use std::fmt::Debug;
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

trait Message<T> {
    fn listeners(inner: &Inner) -> &RefCell<Listeners<T>>;
}

type Listeners<T> = HashMap<String, oneshot::Sender<T>>;

struct Inner {
    promises: RefCell<Vec<Promise>>,
    listeners_string: RefCell<Listeners<String>>,
    listeners_i32: RefCell<Listeners<i32>>,
}

impl Inner {
    fn new() -> Self {
        Self {
            promises: RefCell::new(Vec::new()),
            listeners_string: RefCell::new(HashMap::new()),
            listeners_i32: RefCell::new(HashMap::new()),
        }
    }

    async fn listen<T: Message<T> + Debug>(&self, val: String) -> T {
        let (tx, rx) = oneshot::channel::<T>();
        T::listeners(self).borrow_mut().insert(val, tx);
        rx.await.unwrap()
    }

    fn fire<T: Message<T> + Debug>(&self, name: &str, val: T) {
        T::listeners(self)
            .borrow_mut()
            .remove(name)
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
                self.inner.fire("b", 42);
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

impl Message<String> for String {
    fn listeners(inner: &Inner) -> &RefCell<Listeners<Self>> {
        &inner.listeners_string
    }
}

impl Message<i32> for i32 {
    fn listeners(inner: &Inner) -> &RefCell<Listeners<Self>> {
        &inner.listeners_i32
    }
}

async fn start() -> Result<()> {
    let dispatcher = Dispatcher::new();

    dispatcher.add(Box::new(|d| {
        Box::pin(
            async move {
                let v: String = d.listen(String::from("a")).await;
                dbg!(v);
                Ok(())
            }
            .fuse(),
        )
    }));

    dispatcher.add(Box::new(|d| {
        Box::pin(
            async move {
                let v: i32 = d.listen(String::from("b")).await;
                dbg!(v);
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
