use crate::actors::envelope::Letter;
use crate::services::envelope::ServiceLetterWithResponders;
use crate::services::handle::Notify;
use crate::services::handle::Serve;
use crate::services::manager::{Manager, ServiceManager, ServiceManagerCommand};
use crate::system_director::SystemDirector;
use crate::Service;
use async_std::sync::{channel, Arc, Mutex, Sender};
use dashmap::{mapref::entry::Entry, DashMap};
use futures::task::AtomicWaker;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::{
    any::TypeId,
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

// TODO: This structure is getting big and with several responsiblities, maybe it should be splitted.
#[derive(Debug)]
pub(crate) struct ServicesDirector {
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    // TODO: Should be a WakerSet as there may be more than one thread that wants to wait
    waker: Arc<AtomicWaker>,
    is_stopping: Arc<AtomicBool>,
    system: Arc<Mutex<Option<SystemDirector>>>,
}

impl ServicesDirector {
    pub(crate) fn new() -> ServicesDirector {
        ServicesDirector {
            managers: Arc::new(DashMap::new()),
            waker: Arc::new(AtomicWaker::new()),
            is_stopping: Arc::new(AtomicBool::new(false)),
            system: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) async fn set_system(&mut self, system_director: SystemDirector) {
        let mut system = self.system.lock().await;

        if system.is_none() {
            system.replace(system_director);
        } else {
            unreachable!();
        }
    }

    // Ensures that there is a manager for that type and returns a sender to it
    async fn get_or_create_manager_sender<S: Service>(&self) -> Sender<ServiceManagerCommand<S>> {
        let type_id = TypeId::of::<S>();

        let managers_entry = self.managers.entry(type_id);

        let any_sender = match managers_entry {
            Entry::Occupied(entry) => entry.into_ref(),
            Entry::Vacant(entry) => {
                let manager = self.create_manager::<S>().await;
                entry.insert(Box::new(manager))
            }
        }
        .get_sender_as_any()
        .await;

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

    pub(crate) async fn send<S: Service + Notify<M>, M: Debug + Send + 'static>(&self, message: M) {
        self.get_or_create_manager_sender::<S>()
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

        self.get_or_create_manager_sender::<A>()
            .await
            .send(ServiceManagerCommand::Dispatch(Box::new(
                ServiceLetterWithResponders::new(message, sender),
            )))
            .await;

        receiver.recv().await.ok_or("Ups!")
    }

    pub(crate) async fn wait_until_stopped(&self) {
        ServicesDirectorStopAwaiter::new(self.clone()).await;
    }

    pub(crate) async fn create_manager<S: Service>(&self) -> ServiceManager<S> {
        // We use unwrap here as we must guarantee that there is a system director in every other director
        let system = if let Some(system) = &*self.system.lock().await {
            system.clone()
        } else {
            unreachable!();
        };

        ServiceManager::<S>::new(self.clone(), system).await
    }

    pub(crate) async fn signal_manager_removed(&self) {
        let is_stopping = self.is_stopping.load(Relaxed);
        let is_empty = self.managers.is_empty();

        if is_stopping && is_empty {
            self.waker.wake();
        }
    }

    pub(crate) async fn stop(&self) {
        self.is_stopping.store(true, Relaxed);

        for manager in self.managers.iter() {
            manager.end();
        }
    }

    pub(crate) fn get_blocking_manager_entry(&self, id: TypeId) -> Entry<TypeId, Box<dyn Manager>> {
        self.managers.entry(id)
    }
}

impl Clone for ServicesDirector {
    fn clone(&self) -> Self {
        ServicesDirector {
            managers: self.managers.clone(),
            waker: self.waker.clone(),
            is_stopping: self.is_stopping.clone(),
            system: self.system.clone(),
        }
    }
}

pub(crate) struct ServicesDirectorStopAwaiter(ServicesDirector);

impl ServicesDirectorStopAwaiter {
    pub fn new(waker: ServicesDirector) -> ServicesDirectorStopAwaiter {
        ServicesDirectorStopAwaiter(waker)
    }
}

impl Future for ServicesDirectorStopAwaiter {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if !self.0.is_stopping.load(Relaxed) || !self.0.managers.is_empty() {
            self.0.waker.register(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
