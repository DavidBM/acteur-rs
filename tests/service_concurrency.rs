use acteur::{Acteur, Serve, Service, ServiceAssistant, ServiceConcurrency, ServiceConfiguration};
use async_std::{future::timeout, task::block_on};
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
    let result = block_on(timeout(
        Duration::from_secs(5),
        system.call_service::<Echo, _>(42),
    ));
    assert_eq!(result.expect("service should respond").unwrap(), 42);
    system.stop();
    system.wait_until_stopped();
}
