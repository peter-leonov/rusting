use anyhow::Result;
use futures::executor::block_on;
use futures::future::FusedFuture;
use futures::select;
use futures::Future;
use futures::{
    future::FutureExt, // for `.fuse()`
    pin_mut,
};
use std::pin::Pin;

type Closure = Box<dyn Fn(String) -> Pin<Box<dyn FusedFuture<Output = Result<()>>>>>;

struct Dispatcher {
    listeners: Vec<Closure>,
}

impl Dispatcher {
    fn add(&mut self, cb: Closure) {
        self.listeners.push(cb);
    }

    async fn run(&mut self) -> Result<()> {
        for f in &self.listeners {
            f("aaa".into()).await?;
        }

        Ok(())
    }
}

async fn start() -> Result<()> {
    let mut dispatcher = Dispatcher {
        listeners: Vec::new(),
    };

    dispatcher.add(Box::new(|body| {
        Box::pin(
            async {
                dbg!(body);
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
