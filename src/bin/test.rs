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

    fn listen<T: Message<T> + Debug>(&self, val: String) -> impl Future<Output = T> {
        let (tx, rx) = oneshot::channel::<T>();
        T::listeners(self).borrow_mut().insert(val, tx);
        async { rx.await.unwrap() }
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

type Task = Pin<Box<dyn Future<Output = ()>>>;

struct Tasks(RefCell<Vec<Option<Task>>>);

impl Tasks {
    fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    fn spawn(&self, task: Task) {
        self.0.borrow_mut().push(Some(task))
    }

    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn take(&self, n: usize) -> Option<Task> {
        self.0.borrow_mut()[n].take()
    }

    fn put(&self, n: usize, task: Task) -> Option<Task> {
        self.0.borrow_mut()[n].replace(task)
    }
}

struct Runner {
    tasks: Rc<Tasks>,
}

impl Runner {
    fn new() -> Self {
        Self {
            tasks: Rc::new(Tasks::new()),
        }
    }

    fn tasks(&self) -> Rc<Tasks> {
        self.tasks.clone()
    }
}

use core::task::{Context, Poll};

impl Future for Runner {
    // type Output = F::Output;
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: either purge or come up with a better sructure than ever growing Vec
        let len = self.tasks.len();
        let mut pending = false;
        for i in 0..len {
            let o = self.tasks.take(i);
            if let Some(mut f) = o {
                match f.as_mut().poll(cx) {
                    Poll::Ready(_) => {
                        // dbg!("it's ready");
                    }
                    Poll::Pending => {
                        pending = true;
                        // o must be None here
                        self.tasks.put(i, f);
                    }
                }
            }
        }

        let new_len = self.tasks.len();
        assert!(new_len >= len);
        if new_len > len {
            // dbg!("grew");
            self.poll(cx)
        } else {
            if pending {
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        }
    }
}

async fn start() -> Result<()> {
    let d = Rc::new(Dispatcher::new());

    let runner = Runner::new();
    let t = runner.tasks();

    t.spawn({
        let d = d.clone();
        Box::pin(async move {
            let v: String = d.listen(String::from("a")).await;
            dbg!(v);
        })
    });

    t.spawn({
        let d = d.clone();
        Box::pin(async move {
            let v: String = d.listen(String::from("b")).await;
            dbg!(v);
        })
    });

    t.spawn({
        let d = d.clone();
        Box::pin(async move {
            d.fire("a", String::from("a"));
            my_sleep(0.5).await;
            d.fire("b", String::from("b"));
        })
    });

    t.spawn({
        let d = d.clone();
        Box::pin(async move {
            let p = d.listen(String::from("c"));
            d.fire("c", 42);
            let v: i32 = p.await;
            dbg!(v);
        })
    });

    t.spawn(Box::pin(async {
        dbg!(1);
    }));

    t.spawn({
        let t = t.clone();
        Box::pin(async move {
            dbg!(2);

            t.spawn(Box::pin(async {
                dbg!(4);
            }));
        })
    });

    t.spawn(Box::pin(async {
        dbg!(3);
    }));

    runner.await;

    Ok(())
}

pub fn main() -> Result<()> {
    block_on(start())
}
