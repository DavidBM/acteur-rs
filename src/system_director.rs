use crate::actor_proxy::ActorReport;
use crate::actors_manager::{ActorManagerProxyCommand, ActorsManager, Manager};
use crate::envelope::ManagerLetter;
use crate::{Actor, Handle};
use async_std::{
    sync::{Arc, Sender},
    task,
};
use dashmap::{mapref::entry::Entry, DashMap};
use futures::future::join_all;
use futures::task::AtomicWaker;
use std::{
    any::TypeId,
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

// TODO: This structure is getting big and with several responsiblities, maybe it should be splitted.
#[derive(Debug)]
pub(crate) struct SystemDirector {
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>, // DELETE ?
    // TODO: Should be a WakerSet as there may be more than one thread that wants to wait
    waker: Arc<AtomicWaker>,
}

impl SystemDirector {
    pub(crate) fn new() -> SystemDirector {
        let manager_list = Arc::new(DashMap::new());
        let waker = Arc::new(AtomicWaker::new());

        SystemDirector {
            managers: manager_list.clone(),
            waker: waker.clone(),
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

    pub async fn send<A: Actor + Handle<M>, M: Debug + Send + 'static>(
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

    pub async fn stop_actor<A: Actor>(&self, actor_id: A::Id) {
        self.get_or_create_manager_sender::<A>()
            .send(ActorManagerProxyCommand::EndActor(actor_id))
            .await;
    }

    pub(crate) async fn wait_until_stopped(&self) {
        WaitSystemStop::new(self.clone()).await;
    }

    pub(crate) fn create_manager<A: Actor>(&self) -> ActorsManager<A> {
        ActorsManager::<A>::new(self.clone())
    }

    // TODO: This is wrong. The shutdown method should keep the managers until they report no actors left.
    // The managers should keep the state "ending" and make sure that next time they get 0 actors and 0
    // messages, they report as ended. Same for the system that should just fullfill the future of ending.
    // That or blocking completely any new message sending, but I don't like that idea.
    pub(crate) async fn stop(&self) {
        let mut futures = vec![];
        for manager in self.managers.iter() {
            let manager = self.managers.remove(manager.key()).unwrap().1;
            futures.push(task::spawn(async move {
                manager.end();
            }));
        }

        join_all(futures).await;
    }

    pub(crate) fn get_actor_managers_count(&self) -> usize {
        self.managers.len()
    }

    pub(crate) fn get_statistics(&self) -> Vec<(TypeId, Vec<ActorReport>)> {
        let mut statistics = vec![];

        for manager in self.managers.iter() {
            statistics.push((manager.get_type_id(), manager.get_statistics()))
        }

        statistics
    }
}

impl Clone for SystemDirector {
    fn clone(&self) -> Self {
        SystemDirector {
            managers: self.managers.clone(),
            waker: self.waker.clone(),
        }
    }
}

pub(crate) struct WaitSystemStop(SystemDirector);

impl WaitSystemStop {
    pub fn new(waker: SystemDirector) -> WaitSystemStop {
        WaitSystemStop(waker)
    }
}

impl Future for WaitSystemStop {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.0.get_actor_managers_count() > 0 {
            self.0.waker.register(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
