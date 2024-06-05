use core::future::Future;
use std::pin::Pin;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{ptr, task};

// copypasta from Waker::noop()
const VTABLE: task::RawWakerVTable = task::RawWakerVTable::new(
    // Cloning just returns a new no-op raw waker
    |_| {
        dbg!("cloning my waker");
        RAW
    },
    // `wake` does nothing
    |_| {
        dbg!("waking my waker");
    },
    // `wake_by_ref` does nothing
    |_| {
        dbg!("waking by ref my waker");
    },
    // Dropping does nothing as we don't allocate anything
    |_| {
        dbg!("dropping my waker");
    },
);

const RAW: task::RawWaker = task::RawWaker::new(ptr::null(), &VTABLE);

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

async fn start() -> i32 {
    LazyTimer::new(Duration::from_secs_f32(1.0)).await;
    555
}

fn main() {
    // let waker = Waker::from(&thought);
    // let cx = Context::from_waker(&waker);

    // TODO: try LocalWaker when it's out of nightly
    let waker = unsafe { task::Waker::from_raw(RAW) };

    let mut cx = task::Context::from_waker(&waker);

    let fut = start();
    // TODO: find a way to avoid heap allocation
    let mut fut_pin = Box::pin(fut);
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
