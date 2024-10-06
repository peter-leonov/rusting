use core::future::Future;
use std::pin;
use std::pin::Pin;
use std::sync::Arc;
use std::task;
use std::thread::sleep;
use std::time::{Duration, Instant};

struct MyWaker {
    flag: bool,
}

impl MyWaker {
    fn new() -> Self {
        Self { flag: false }
    }
}

impl task::Wake for MyWaker {
    fn wake(self: Arc<Self>) {
        dbg!("waking the waker does nothing");
        // self.0.unpark();
    }
}

struct LazyTimer {
    due: Instant,
}

impl LazyTimer {
    fn new(sleep_for: Duration) -> Self {
        Self {
            due: Instant::now() + sleep_for,
        }
    }
}

impl Future for LazyTimer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if Instant::now() >= self.due {
            task::Poll::Ready(())
        } else {
            task::Poll::Pending
        }
    }
}

// The plan.
// Simple next steps:
// * support multiple concurrent timers that efficiently use a queue
// * first teach the runtime to sleep exactly right for a single timer

// Multi-threading:
// * avoid parking the thread and using the waker for as long a possible to have a good demonstration of how things really work
// * implement a Promise that when `.resolve()`ed wakes the running thread up and returns the resolved value
// * when resolved withing the same thread Promise has to indicate to the executor to run the poll() again (a microtask sort of thing)

async fn start() {
    dbg!("start {");
    LazyTimer::new(Duration::from_secs_f32(0.5)).await;
    dbg!("start }");
}

fn main() {
    let waker = Arc::new(MyWaker::new()).into();
    let mut cx = task::Context::from_waker(&waker);

    let fut = start();
    let mut fut_pin = pin::pin!(fut);
    loop {
        if let task::Poll::Ready(v) = fut_pin.as_mut().poll(&mut cx) {
            dbg!(v);
            break;
        }
        dbg!("sleep a bit");
        sleep(Duration::from_secs_f32(0.1));
    }
}
