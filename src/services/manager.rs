use crate::services::handle::Notify;
use crate::services::envelope::ServiceEnvelope;
use std::sync::atomic::{AtomicUsize};
use async_std::{sync::{Arc, channel, Sender, Receiver}, task};
use crate::services::service::{Service, ServiceConcurrency};

#[derive(Debug)]
struct ServiceManager<S: Service> {
    senders: Vec<Sender<dyn ServiceEnvelope>>,
    current: AtomicUsize,
}

impl <S: Service> ServiceManager<S> {
    async fn new() -> ServiceManager<S> {
        let (service, service_conf) = S::initialize().await;

        let service = Arc::new(service);

        let concurrency = match service_conf.concurrency {
            ServiceConcurrency::Automatic => num_cpus::get(),
            ServiceConcurrency::None => 1,
            ServiceConcurrency::OnePerCore => num_cpus::get(),
            ServiceConcurrency::OneEachTwoCore => num_cpus::get() / 2,
            ServiceConcurrency::Fixed(quantity) => quantity,
        };


        let mut senders = Vec::new();

        for _ in concurrency {
            let (sender, receiver) = channel::<dyn ServiceEnvelope>(150_000);
            senders.push(sender);

            service_loop(receiver, service.clone());
        }

        ServiceManager {
            senders,
            current: AtomicUsize::new(0),
        }
    }

    async fn send<M>(&self, message: M)
    where S: Notify<M> {
        let current = {
            let current = self.current.get_mut();
            
            *current = current + 1;

            if current >= self.senders.len() {
                *current = 0;
            }

            current.clone()
        };

        let sender = match self.senders.get(current) {
            Some(sender) => sender,
            _ => unreachable!(),
        };

        sender.send().await
    }
}

fn service_loop<S>(receiver: Receiver<dyn ServiceEnvelope>, service: Arc<ServiceManager<S>>) {
    task::spawn(async move {
        while let Some(message) = receiver.recv().await {
            match message {

            }
        }
    });
}
