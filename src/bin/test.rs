// Not using tokio to understand futures and async/await in Rust better.
// Not writing tests to not make it more bureaucratic than necessary.
use anyhow::{anyhow, Result};
use futures::executor::block_on;
use futures::Future;
use futures::{channel::oneshot, TryFutureExt};
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

    fn listen<T: Message<T> + Debug>(&self, val: String) -> impl Future<Output = Result<T>> {
        let (tx, rx) = oneshot::channel::<T>();
        T::listeners(self).borrow_mut().insert(val, tx);
        rx.or_else(|_| async { Err(anyhow!("error receiving a message")) })
    }

    fn fire<T: Message<T> + Debug>(&self, name: &str, val: T) -> Result<bool> {
        if let Some(tx) = T::listeners(self).borrow_mut().remove(name) {
            if let Err(val) = tx.send(val) {
                return Err(anyhow!("error firing a message {:?}", val));
            }
            Ok(true)
        } else {
            Ok(false)
        }
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

type Task = Pin<Box<dyn Future<Output = Result<()>>>>;

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

    // fn spawn_task(&self, task: Task) {
    //     self.new.borrow_mut().push(task)
    // }

    // for why and what 'static is see:
    // https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md#2-if-t-static-then-t-must-be-valid-for-the-entire-program
    fn spawn<F: Future<Output = Result<()>> + 'static>(&self, fut: F) {
        // Seems like it's safe to pass a nonstarted future around.
        self.new.borrow_mut().push(Box::pin(fut))
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
        async move {
            dbg!("async 1 {");
            let v: String = d.listen(String::from("a")).await?;
            assert!(v == String::from("value for a"));
            dbg!("async 1 }");
            Ok(())
        }
    });

    t.spawn({
        let d = d.clone();
        async move {
            dbg!("async 2 {");
            let v: String = d.listen(String::from("b")).await?;
            assert!(v == String::from("value for b"));
            dbg!("async 2 }");
            Ok(())
        }
    });

    t.spawn({
        let d = d.clone();
        async move {
            dbg!("async 3 {");
            d.fire("a", String::from("value for a"))?;
            my_sleep(0.1).await;
            d.fire("b", String::from("value for b"))?;
            dbg!("async 3 }");
            Ok(())
        }
    });

    t.spawn({
        let d = d.clone();
        async move {
            dbg!("async 4 {");
            let p = d.listen(String::from("c"));
            d.fire("c", 42)?;
            let v: i32 = p.await?;
            assert!(v == 42);
            dbg!("async 4 }");
            Ok(())
        }
    });

    t.spawn(async {
        dbg!("async 5 {");
        dbg!("async 5 }");
        Ok(())
    });

    t.spawn({
        let t = t.clone();
        async move {
            dbg!("async 6 {");
            t.spawn(async {
                dbg!("async 7 {");
                dbg!("async 7 }");
                Ok(())
            });
            dbg!("async 6 }");
            Ok(())
        }
    });

    t.spawn(async {
        dbg!("async 8 {");
        dbg!("async 8 }");
        Ok(())
    });

    runner.await;

    Ok(())
}

pub fn main() -> Result<()> {
    block_on(start())
}
