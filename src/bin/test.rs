use anyhow::Result;
use futures::channel::oneshot;
use futures::executor::block_on;
use futures::future::join_all;
use futures::Future;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
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

type Promise = Pin<Box<dyn Future<Output = ()>>>;

trait Message<T> {
    fn listeners(inner: &Dispatcher) -> &RefCell<Listeners<T>>;
}

type Listeners<T> = HashMap<String, oneshot::Sender<T>>;

struct Dispatcher {
    listeners_string: RefCell<Listeners<String>>,
    listeners_i32: RefCell<Listeners<i32>>,
}

impl Dispatcher {
    fn new() -> Self {
        Self {
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

impl Message<String> for String {
    fn listeners(inner: &Dispatcher) -> &RefCell<Listeners<Self>> {
        &inner.listeners_string
    }
}

impl Message<i32> for i32 {
    fn listeners(inner: &Dispatcher) -> &RefCell<Listeners<Self>> {
        &inner.listeners_i32
    }
}

async fn start() -> Result<()> {
    let d = Rc::new(Dispatcher::new());

    let d1 = d.clone();
    let d2 = d.clone();

    let futures = vec![
        Box::pin(async move {
            let v: String = d1.listen(String::from("a")).await;
            dbg!(v);
        }) as Promise,
        Box::pin(async move {
            let v: i32 = d2.listen(String::from("b")).await;
            dbg!(v);
        }) as Promise,
        Box::pin(async move {
            my_sleep(0.5).await;
            d.fire("b", 42);
            my_sleep(0.5).await;
            d.fire("a", String::from("aaa"));
        }) as Promise,
    ];

    join_all(futures).await;

    Ok(())
}

pub fn main() -> Result<()> {
    block_on(start())
}
