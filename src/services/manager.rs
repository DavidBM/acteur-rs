use crate::services::broker::MessageBroker;
use crate::services::director::ServicesDirector;
use crate::services::envelope::ServiceEnvelope;
use crate::services::service::{Service, ServiceConcurrency};
use crate::services::system_facade::ServiceAssistant;
use crate::system_director::SystemDirector;
use async_channel::{unbounded as channel, Receiver, Sender};
use async_std::sync::Mutex;
use async_std::{sync::Arc, task};
use dashmap::mapref::entry::Entry::Occupied;
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

#[async_trait::async_trait]
pub(crate) trait Manager: Send + Sync + Debug {
    fn end(&self);
    fn get_type_id(&self) -> TypeId;
    async fn get_sender_as_any(&mut self) -> Box<(dyn Any + Send)>;
    fn get_statistics(&self) -> ServiceReport;
    fn clone(&self) -> Box<dyn Manager>;
}

pub struct ServiceReport {
    pub last_message_on: SystemTime,
    pub enqueued_messages: usize,
}

#[derive(Debug)]
pub(crate) enum ServiceManagerCommand<S: Service> {
    Dispatch(Box<dyn ServiceEnvelope<Service = S>>),
    End,
}

#[derive(Debug)]
pub(crate) struct ServiceManager<S: Service> {
    senders: Arc<Vec<Sender<ServiceManagerCommand<S>>>>,
    current: Arc<Mutex<usize>>,
    is_ending: Arc<AtomicBool>,
    active_services: Arc<AtomicUsize>,
}

impl<S: Service> ServiceManager<S> {
    pub async fn new(
        director: ServicesDirector,
        system_director: SystemDirector,
        broker: MessageBroker,
    ) -> ServiceManager<S> {
        let system_facade = ServiceAssistant::<S>::new(system_director.clone(), broker.clone());

        let (service, service_conf) = S::initialize(&system_facade).await;

        let service = Arc::new(service);

        // Controls if the loop waits for the service functions to finish.
        let mut wait_for_service = true;

        let concurrency = match service_conf.concurrency {
            ServiceConcurrency::Automatic => {
                // If the structure is size 0 we can safely assume that there is no state / synchronization mechanisms,
                // therefore we set the concurrency as unlimited.
                if std::mem::size_of::<S>() == 0 {
                    // loop won't wait for service handler to finish
                    wait_for_service = false;
                    1
                } else {
                    num_cpus::get()
                }
            }
            ServiceConcurrency::None => 1,
            ServiceConcurrency::OnePerCore => num_cpus::get(),
            ServiceConcurrency::OneEachTwoCore => num_cpus::get() / 2,
            ServiceConcurrency::Fixed(quantity) => quantity,
            ServiceConcurrency::Unlimited => {
                // loop won't wait for service handler to finish
                wait_for_service = false;
                1
            }
        };

        let mut senders = Vec::new();
        let mut receivers = Vec::new();

        for _ in 0..concurrency {
            let (sender, receiver) = channel::<ServiceManagerCommand<S>>();
            senders.push(sender);
            receivers.push(receiver);
        }

        let senders = Arc::new(senders);
        let current = Arc::new(Mutex::new(0));
        let active_services = Arc::new(AtomicUsize::new(concurrency));

        let manager = ServiceManager {
            senders: senders.clone(),
            current,
            is_ending: Arc::new(AtomicBool::new(false)),
            active_services: active_services.clone(),
        };

        for (receiver, sender) in receivers.iter().zip(senders.as_ref()) {
            service_loop(
                receiver.clone(),
                sender.clone(),
                service.clone(),
                director.clone(),
                active_services.clone(),
                system_director.clone(),
                broker.clone(),
                wait_for_service,
            );
        }

        manager
    }

    pub(crate) async fn get_sender(&self) -> Sender<ServiceManagerCommand<S>> {
        let current = self.get_next_sender_index().await;

        match self.senders.get(current) {
            Some(sender) => sender.clone(),
            _ => unreachable!(),
        }
    }

    pub(crate) async fn get_next_sender_index(&self) -> usize {
        if self.senders.len() == 1 {
            return 0;
        }

        let mut current = self.current.lock().await;

        *current += 1;

        if *current >= self.senders.len() {
            *current = 0;
        }

        *current
    }

    fn end(&self) {
        for sender in self.senders.iter() {
            let sender = sender.clone();
            task::spawn(async move {
                // The channel is unbounded and it is ok if it is closed.
                // That is the reason for ignoring the error
                let _ = sender.send(ServiceManagerCommand::End).await;
            });
        }
    }
}

fn service_loop<S: Service>(
    receiver: Receiver<ServiceManagerCommand<S>>,
    sender: Sender<ServiceManagerCommand<S>>,
    service: Arc<S>,
    director: ServicesDirector,
    active_services: Arc<AtomicUsize>,
    system_director: SystemDirector,
    broker: MessageBroker,
    wait_for_service: bool,
) {
    task::spawn(async move {
        let system_facade = Arc::new(ServiceAssistant::<S>::new(system_director, broker));

        loop {
            if let Ok(command) = receiver.recv().await {
                match command {
                    ServiceManagerCommand::Dispatch(envelope) => {
                        dispatch::<S>(&service, &system_facade, envelope, wait_for_service).await;
                    }
                    // This algorithm is basically the same as the one in the Actor's Proxy. Check that file
                    // for an explanation in detail.
                    //
                    // Basically, we are trying to consume all End commands (even if there are several in a row)
                    // and if we find another command that is not end, requeue the end and process such command.
                    ServiceManagerCommand::End => {
                        match recv_until_command_or_end!(receiver, ServiceManagerCommand::End).await
                        {
                            None | Some(ServiceManagerCommand::End) => {
                                // From here to the `break;` statement only 1 thread will do it at the same time
                                // as this line will block other threads.
                                let entry = director
                                    .get_blocking_manager_entry(std::any::TypeId::of::<S>());

                                // Now that we are sure that only one threat is here at a time, lets see if there
                                // are more messages pending.
                                match recv_until_command_or_end!(
                                    receiver,
                                    ServiceManagerCommand::End
                                )
                                .await
                                {
                                    // If there are more messages, we requeue the end and process the message.
                                    Some(ServiceManagerCommand::Dispatch(envelope)) => {
                                        drop(entry);
                                        let _ = sender.send(ServiceManagerCommand::End).await;
                                        dispatch::<S>(
                                            &service,
                                            &system_facade,
                                            envelope,
                                            wait_for_service,
                                        )
                                        .await;
                                    }
                                    // If there aren't new messages, we finish the loop.
                                    None | Some(ServiceManagerCommand::End) => {
                                        // Given that services run with some concurrency, we keep the count
                                        // of services actually running.
                                        let previously_active =
                                            active_services.fetch_sub(1, Ordering::Relaxed);

                                        if let Occupied(entry) = entry {
                                            // Only if there are 0 we remove the manager.
                                            // We check agains 1 because fetch_sub returns the previous number.
                                            if previously_active <= 1 {
                                                entry.remove();
                                                director.signal_manager_removed().await;
                                            }
                                        }

                                        break;
                                    }
                                }
                            }
                            Some(ServiceManagerCommand::Dispatch(envelope)) => {
                                let _ = sender.send(ServiceManagerCommand::End).await;
                                dispatch::<S>(&service, &system_facade, envelope, wait_for_service)
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    });
}

async fn dispatch<'a, S: Service>(
    service: &'a Arc<S>,
    system_facade: &'a Arc<ServiceAssistant<S>>,
    mut envelope: Box<dyn ServiceEnvelope<Service = S>>,
    wait_for_service: bool,
) {
    if wait_for_service {
        envelope.dispatch(&service, &system_facade).await;
    } else {
        let service = service.clone();
        let system_facade = system_facade.clone();
        task::spawn(async move { envelope.dispatch(&service, &system_facade).await });
    }
}

#[async_trait::async_trait]
impl<S: Service> Manager for ServiceManager<S> {
    async fn get_sender_as_any(&mut self) -> Box<(dyn Any + Send + 'static)> {
        Box::new(self.get_sender().await)
    }
    fn get_type_id(&self) -> TypeId {
        std::any::TypeId::of::<S>()
    }
    fn end(&self) {
        self.end();
    }

    // TODO: Implement
    fn get_statistics(&self) -> ServiceReport {
        ServiceReport {
            last_message_on: SystemTime::now(),
            enqueued_messages: 10000,
        }
    }

    fn clone(&self) -> Box<dyn Manager> {
        Box::new(Clone::clone(self))
    }
}

impl<S: Service> Clone for ServiceManager<S> {
    fn clone(&self) -> ServiceManager<S> {
        ServiceManager {
            senders: self.senders.clone(),
            current: self.current.clone(),
            is_ending: self.is_ending.clone(),
            active_services: self.active_services.clone(),
        }
    }
}
