use std::fmt::Debug;
use crate::services::handle::Notify;
use crate::actors::envelope::Letter;
use crate::services::envelope::ServiceEnvelope;
use std::sync::atomic::{AtomicUsize};
use async_std::{sync::{Arc, channel, Sender, Receiver}, task};
use crate::services::service::{Service, ServiceConcurrency};


#[derive(Debug)]
struct ServiceManager<S: Service> {
    senders: Vec<Sender<Box<dyn ServiceEnvelope<Service = S>>>>,
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

        for _ in 0..concurrency {
            let (sender, receiver) = channel::<Box<dyn ServiceEnvelope<Service = S>>>(150_000);
            senders.push(sender);

            service_loop(receiver, service.clone());
        }

        ServiceManager {
            senders,
            current: AtomicUsize::new(0),
        }
    }

    async fn send<M: Debug + Send + 'static>(&mut self, message: M)
    where S: Service + Notify<M>{
        let current = {
            let current = self.current.get_mut();
            
            *current += 1;

            if current >= &mut self.senders.len() {
                *current = 0;
            }

            // I think it is cleaner is clone is called specifically.
            #[allow(clippy::clone_on_copy)]
            current.clone()
        };

        let sender = match self.senders.get(current) {
            Some(sender) => sender,
            _ => unreachable!(),
        };

        let message = Letter::<S, M>::new_for_service(message);

        sender.send(Box::new(message)).await;
    }
}

fn service_loop<S: Service>(receiver: Receiver<Box<dyn ServiceEnvelope<Service = S>>>, _service: Arc<S>) {
    task::spawn(async move {
        while let Some(_message) = receiver.recv().await {
        }
    });
}
