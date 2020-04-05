use crate::system_director::SystemDirector;
use crate::actors::envelope::{ManagerLetter, ManagerLetterWithResponder};
use crate::actors::manager::{ActorManagerProxyCommand, ActorsManager, Manager};
use crate::actors::proxy::ActorReport;
use crate::{Actor, Receive, Respond};
use async_std::sync::channel;
use async_std::sync::{Arc, Sender};
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
pub(crate) struct ActorsDirector {
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    // TODO: Should be a WakerSet as there may be more than one thread that wants to wait
    waker: Arc<AtomicWaker>,
    is_stopping: Arc<AtomicBool>,
    system: Arc<Option<SystemDirector>>,
}

impl ActorsDirector {
    pub(crate) fn new() -> ActorsDirector {
        ActorsDirector {
            managers: Arc::new(DashMap::new()),
            waker: Arc::new(AtomicWaker::new()),
            is_stopping: Arc::new(AtomicBool::new(false)),
            system: Arc::new(None),
        }
    }

    pub(crate) fn set_system(&mut self, system: SystemDirector) {
        if self.system.is_none() {
            if let Some(old_system) = Arc::get_mut(&mut self.system) {
                *old_system = Some(system);
            } else {
                unreachable!();
            }
        } else {
            unreachable!();
        }
    }

    // Ensures that there is a manager for that type and returns a sender to it
    fn get_or_create_manager_sender<A: Actor>(&self) -> Sender<ActorManagerProxyCommand<A>> {
        let type_id = TypeId::of::<A>();

        let managers_entry = self.managers.entry(type_id);

        let any_sender = match managers_entry {
            Entry::Occupied(entry) => entry.into_ref(),
            Entry::Vacant(entry) => {
                let manager = self.create_manager::<A>();
                entry.insert(Box::new(manager))
            }
        }
        .get_sender_as_any();

        match any_sender.downcast::<Sender<ActorManagerProxyCommand<A>>>() {
            Ok(sender) => *sender,
            // If type is not matching, crash as  we don't really want to
            // run the framework with a bug like that
            Err(_) => unreachable!(),
        }
    }

    pub(crate) async fn send<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        self.get_or_create_manager_sender::<A>()
            .send(ActorManagerProxyCommand::Dispatch(Box::new(
                ManagerLetter::new(actor_id, message),
            )))
            .await;
    }

    // TODO: Create a proper return type without &str
    pub(crate) async fn call<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        let (sender, receiver) = channel::<<A as Respond<M>>::Response>(1);

        self.get_or_create_manager_sender::<A>()
            .send(ActorManagerProxyCommand::Dispatch(Box::new(
                ManagerLetterWithResponder::new(actor_id, message, sender),
            )))
            .await;

        receiver.recv().await.ok_or("Ups!")
    }

    pub(crate) async fn stop_actor<A: Actor>(&self, actor_id: A::Id) {
        self.get_or_create_manager_sender::<A>()
            .send(ActorManagerProxyCommand::EndActor(actor_id))
            .await;
    }

    pub(crate) async fn wait_until_stopped(&self) {
        ActorsDirectorStopAwaiter::new(self.clone()).await;
    }

    pub(crate) fn create_manager<A: Actor>(&self) -> ActorsManager<A> {
        // We use unwrap here as we must guarantee that there is a system director in every other director
        ActorsManager::<A>::new(self.clone(), self.system.as_ref().as_ref().unwrap().clone())
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

    pub(crate) fn get_statistics(&self) -> Vec<(TypeId, Vec<ActorReport>)> {
        let mut statistics = vec![];

        for manager in self.managers.iter() {
            statistics.push((manager.get_type_id(), manager.get_statistics()))
        }

        statistics
    }
}

impl Clone for ActorsDirector {
    fn clone(&self) -> Self {
        ActorsDirector {
            managers: self.managers.clone(),
            waker: self.waker.clone(),
            is_stopping: self.is_stopping.clone(),
            system: self.system.clone(),
        }
    }
}

pub(crate) struct ActorsDirectorStopAwaiter(ActorsDirector);

impl ActorsDirectorStopAwaiter {
    pub fn new(waker: ActorsDirector) -> ActorsDirectorStopAwaiter {
        ActorsDirectorStopAwaiter(waker)
    }
}

impl Future for ActorsDirectorStopAwaiter {
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
