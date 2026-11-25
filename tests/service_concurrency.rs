use acteur::{Acteur, Serve, Service, ServiceAssistant, ServiceConcurrency, ServiceConfiguration};
use async_global_executor::block_on;
use async_io::Timer;
use futures_lite::future::race;
use std::time::Duration;

#[derive(Debug)]
struct Echo;

#[async_trait::async_trait]
impl Service for Echo {
    async fn initialize(_: &ServiceAssistant<Self>) -> (Self, ServiceConfiguration) {
        (
            Self,
            ServiceConfiguration {
                concurrency: ServiceConcurrency::Fixed(0),
            },
        )
    }
}

#[async_trait::async_trait]
impl Serve<u32> for Echo {
    type Response = u32;

    async fn handle(&self, message: u32, _: &ServiceAssistant<Self>) -> u32 {
        message
    }
}

#[test]
fn zero_configured_workers_still_process_messages() {
    let system = Acteur::new();
    let result = block_on(race(
        async { Some(system.call_service::<Echo, _>(42).await) },
        async {
            Timer::after(Duration::from_secs(5)).await;
            None
        },
    ));
    assert_eq!(result.expect("service should respond").unwrap(), 42);
    system.stop();
    system.wait_until_stopped();
}
