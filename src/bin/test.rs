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
        let promises = mem::replace(self.inner.promises.borrow_mut().as_mut(), Vec::new());
        join_all(promises).await.into_iter().collect()
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

    dispatcher.add(Box::new(|d| {
        Box::pin(
            async move {
                my_sleep(0.5).await;
                d.fire("a", String::from("aaa"));
                my_sleep(0.5).await;
                d.fire("b", 42);
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
