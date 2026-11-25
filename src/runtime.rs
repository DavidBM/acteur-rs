use async_io::Timer;
use futures_lite::future::race;
use std::{future::Future, time::Duration};

pub(crate) use async_global_executor::block_on;

pub(crate) fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    async_global_executor::spawn(future).detach();
}

pub(crate) async fn timeout<T>(
    duration: Duration,
    future: impl Future<Output = T>,
) -> Result<T, ()> {
    race(async { Ok(future.await) }, async {
        Timer::after(duration).await;
        Err(())
    })
    .await
}
