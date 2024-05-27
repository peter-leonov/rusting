// Not using tokio to understand futures and async/await in Rust better.
// Not writing tests to not make it more bureaucratic than necessary.
use anyhow::Result;
use futures::channel::oneshot;
use futures::executor::block_on;
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

struct Tasks {
    old: RefCell<Vec<Task>>,
    new: RefCell<Vec<Task>>,
}

impl Tasks {
    fn new() -> Self {
        Self {
            old: RefCell::new(Vec::new()),
            new: RefCell::new(Vec::new()),
        }
    }

    fn spawn(&self, task: Task) {
        self.new.borrow_mut().push(task)
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

fn poll_vec(tasks: &mut Vec<Task>, cx: &mut Context<'_>) -> bool {
    tasks.retain_mut(|task| match task.as_mut().poll(cx) {
        Poll::Ready(_) => false,
        Poll::Pending => true,
    });
    tasks.is_empty()
}

impl Future for Runner {
    // type Output = F::Output;
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut old_empty = poll_vec(self.tasks.old.borrow_mut().as_mut(), cx);

        loop {
            let mut new = self.tasks.new.take();
            let new_empty = poll_vec(&mut new, cx);
            if new_empty {
                break;
            }
            self.tasks.old.borrow_mut().append(&mut new);
            old_empty = false;
        }

        if old_empty {
            Poll::Ready(())
        } else {
            Poll::Pending
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
            dbg!("async 1");
        })
    });

    t.spawn({
        let d = d.clone();
        Box::pin(async move {
            let v: String = d.listen(String::from("b")).await;
            dbg!(v);
            dbg!("async 2");
        })
    });

    t.spawn({
        let d = d.clone();
        Box::pin(async move {
            d.fire("a", String::from("value for a"));
            dbg!("before sleep");
            my_sleep(0.1).await;
            dbg!("after sleep");
            d.fire("b", String::from("value for b"));
            dbg!("async 3");
        })
    });

    t.spawn({
        let d = d.clone();
        Box::pin(async move {
            let p = d.listen(String::from("c"));
            d.fire("c", String::from("value for c"));
            let v: String = p.await;
            dbg!(v);
            dbg!("async 4");
        })
    });

    t.spawn(Box::pin(async {
        dbg!("simple one");
        dbg!("async 5");
    }));

    t.spawn({
        let t = t.clone();
        Box::pin(async move {
            dbg!("adding a dynamic task");

            t.spawn(Box::pin(async {
                dbg!("in a dynamic task");
                dbg!("async 7");
            }));
            dbg!("async 6");
        })
    });

    t.spawn(Box::pin(async {
        dbg!("simple two");
        dbg!("async 8");
    }));

    runner.await;

    Ok(())
}

pub fn main() -> Result<()> {
    block_on(start())
}
