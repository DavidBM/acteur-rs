use crate::actors::envelope::Letter;
use crate::services::broker::MessageBroker;
use crate::services::envelope::ServiceLetterWithResponders;
use crate::services::handle::Listen;
use crate::services::handle::Serve;
use crate::services::manager::{Manager, ServiceManager, ServiceManagerCommand};
use crate::system_director::SystemDirector;
use crate::Service;
use async_channel::{bounded as channel, Sender};
use async_lock::Mutex;
use dashmap::mapref::one::RefMut;
use dashmap::{mapref::entry::Entry, DashMap};
use event_listener::Event;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use std::{any::TypeId, fmt::Debug};

// TODO: This structure is getting big and with several responsiblities, maybe it should be splitted.
#[derive(Debug)]
pub(crate) struct ServicesDirector {
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    stopped: Arc<Event>,
    is_stopping: Arc<AtomicBool>,
    system: Arc<Mutex<Option<SystemDirector>>>,
    broker: Option<MessageBroker>,
}

impl ServicesDirector {
    pub(crate) fn new() -> ServicesDirector {
        let mut director = ServicesDirector {
            managers: Arc::new(DashMap::new()),
            stopped: Arc::new(Event::new()),
            is_stopping: Arc::new(AtomicBool::new(false)),
            system: Arc::new(Mutex::new(None)),
            broker: None,
        };

        let broker = MessageBroker::new(director.clone());

        director.broker = Some(broker);

        director
    }

    pub(crate) async fn set_system(&mut self, system_director: SystemDirector) {
        let mut system = self.system.lock().await;

        if system.is_none() {
            system.replace(system_director);
        } else {
            unreachable!();
        }
    }

    async fn get_mamager<S: Service>(&self) -> RefMut<'_, TypeId, Box<dyn Manager>> {
        let type_id = TypeId::of::<S>();

        let managers_entry = self.managers.entry(type_id);

        match managers_entry {
            Entry::Occupied(entry) => entry.into_ref(),
            Entry::Vacant(entry) => {
                let manager = self.create_manager::<S>().await;
                entry.insert(Box::new(manager))
            }
        }
    }

    // Ensures that there is a manager for that type and returns a sender to it
    async fn get_or_create_manager_sender<S: Service>(&self) -> Sender<ServiceManagerCommand<S>> {
        let any_sender = self.get_mamager::<S>().await.get_sender_as_any().await;

        match any_sender.downcast::<Sender<ServiceManagerCommand<S>>>() {
            Ok(sender) => *sender,
            // If type is not matching, crash as  we don't really want to
            // run the framework with a bug like that
            Err(_) => unreachable!(),
        }
    }

    pub(crate) async fn preload<S: Service>(&self) {
        self.get_or_create_manager_sender::<S>().await;
    }

    pub(crate) async fn send<S: Service + Listen<M>, M: Debug + Send + 'static>(&self, message: M) {
        let _ = self
            .get_or_create_manager_sender::<S>()
            .await
            .send(ServiceManagerCommand::Dispatch(Box::new(
                Letter::new_for_service(message),
            )))
            .await;
    }

    // TODO: Create a proper return type without &str
    pub(crate) async fn call<A: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<A as Serve<M>>::Response, &str> {
        let (sender, receiver) = channel::<<A as Serve<M>>::Response>(1);

        let _ = self
            .get_or_create_manager_sender::<A>()
            .await
            .send(ServiceManagerCommand::Dispatch(Box::new(
                ServiceLetterWithResponders::new(message, sender),
            )))
            .await;

        receiver.recv().await.or(Err("Ups"))
    }

    pub(crate) async fn wait_until_stopped(&self) {
        loop {
            let listener = self.stopped.listen();
            if self.is_stopping.load(Relaxed) && self.managers.is_empty() {
                return;
            }
            listener.await;
        }
    }

    pub(crate) async fn create_manager<S: Service>(&self) -> ServiceManager<S> {
        // We use unwrap here as we must guarantee that there is a system director in every other director
        let system = if let Some(system) = &*self.system.lock().await {
            system.clone()
        } else {
            unreachable!();
        };

        ServiceManager::<S>::new(self.clone(), system, self.broker.as_ref().unwrap().clone()).await
    }

    pub(crate) fn signal_manager_removed(&self) {
        let is_stopping = self.is_stopping.load(Relaxed);
        let is_empty = self.managers.is_empty();

        if is_stopping && is_empty {
            self.stopped.notify(usize::MAX);
        }
    }

    pub(crate) async fn stop(&self) {
        self.is_stopping.store(true, Relaxed);

        for manager in self.managers.iter() {
            manager.end();
        }
        self.signal_manager_removed();
    }

    pub(crate) fn get_blocking_manager_entry(
        &self,
        id: TypeId,
    ) -> Entry<'_, TypeId, Box<dyn Manager>> {
        self.managers.entry(id)
    }

    pub(crate) async fn publish<M: Send + Clone + 'static>(&self, message: M) {
        self.broker.as_ref().unwrap().publish(message).await
    }
}

impl Clone for ServicesDirector {
    fn clone(&self) -> Self {
        ServicesDirector {
            managers: self.managers.clone(),
            stopped: self.stopped.clone(),
            is_stopping: self.is_stopping.clone(),
            system: self.system.clone(),
            broker: self.broker.clone(),
        }
    }
}
