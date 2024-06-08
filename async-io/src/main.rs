use core::future::Future;
use std::pin;
use std::pin::Pin;
use std::sync::Arc;
use std::task;
use std::thread::sleep;
use std::time::{Duration, Instant};

struct MyWaker;

impl task::Wake for MyWaker {
    fn wake(self: Arc<Self>) {
        dbg!("waking my waker");
        // self.0.unpark();
    }
}

struct LazyTimer {
    start: Instant,
    sleep_for: Duration,
}

impl LazyTimer {
    fn new(sleep_for: Duration) -> Self {
        Self {
            start: Instant::now(),
            sleep_for,
        }
    }
}

impl Future for LazyTimer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if Instant::now().duration_since(self.start) > self.sleep_for {
            task::Poll::Ready(())
        } else {
            task::Poll::Pending
        }
    }
}

// plan:
// implement a Promise that when `.resolve()`ed wakes the task up and return the resolved value
// when resolved withing the same thread Promise has to indicate the the executor to run the poll() again

async fn start() -> i32 {
    LazyTimer::new(Duration::from_secs_f32(0.5)).await;
    555
}

fn main() {
    let waker = Arc::new(MyWaker).into();
    let mut cx = task::Context::from_waker(&waker);

    let fut = start();
    let mut fut_pin = pin::pin!(fut);
    loop {
        if let task::Poll::Ready(v) = fut_pin.as_mut().poll(&mut cx) {
            dbg!(v);
            break;
        }
        // TODO: actually use the waker to wake the thread up
        dbg!("sleep a bit");
        sleep(Duration::from_secs_f32(0.1));
    }
}
