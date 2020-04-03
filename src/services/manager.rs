use crate::services::envelope::ServiceEnvelope;
use crate::services::service::{Service, ServiceConcurrency};
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task,
};
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use std::sync::atomic::AtomicUsize;

pub(crate) trait Manager {
    fn end(&self);
    fn get_type_id(&self) -> TypeId;
    fn get_statistics(&self) -> ServiceManagerReport;
    fn get_sender_as_any(&mut self) -> Box<dyn Any>;
}

pub(crate) type ServiceManagerReport = Vec<u32>;

#[derive(Debug)]
pub(crate) enum ServiceManagerCommand<S: Service> {
    Dispatch(Box<dyn ServiceEnvelope<Service = S>>),
    End,
}

#[derive(Debug)]
struct ServiceManager<S: Service> {
    senders: Vec<Sender<ServiceManagerCommand<S>>>,
    current: AtomicUsize,
}

impl<S: Service> ServiceManager<S> {
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
            let (sender, receiver) = channel::<ServiceManagerCommand<S>>(150_000);
            senders.push(sender);

            service_loop(receiver, service.clone());
        }

        ServiceManager {
            senders,
            current: AtomicUsize::new(0),
        }
    }

    fn get_sender(&mut self) -> Sender<ServiceManagerCommand<S>> {
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

        match self.senders.get(current) {
            Some(sender) => sender.clone(),
            _ => unreachable!(),
        }
    }
}

fn service_loop<S: Service>(receiver: Receiver<ServiceManagerCommand<S>>, _service: Arc<S>) {
    task::spawn(async move { while let Some(_message) = receiver.recv().await {} });
}

impl<S: Service> Manager for ServiceManager<S> {
    fn get_sender_as_any(&mut self) -> Box<(dyn Any + 'static)> {
        Box::new(self.get_sender())
    }
    fn get_statistics(&self) -> Vec<u32> {
        unimplemented!()
    }
    fn get_type_id(&self) -> TypeId {
        unimplemented!()
    }
    fn end(&self) {
        unimplemented!()
    }
}
