use std::any::Any;
use crate::actor_proxy::{ActorReport};
use crate::actors_manager::{ActorManagerProxyCommand, ActorsManager, Manager, ActorEndResult};
use crate::envelope::ManagerLetter;
use crate::{Actor, Handle};
use async_std::{
    task,
    sync::{Arc, Sender},
};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use futures::{task::AtomicWaker};
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
                let manager = self.create::<A>();
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

    pub async fn stop_actor(&self, type_id: TypeId, actor_id: Box<dyn Any + Send>) {

        if let Some(manager) = self.managers.get(&type_id) {
            match manager.end_actor(Box::new(actor_id)).await {
                ActorEndResult::ActorManagerEmptyAndTerminated => {
                    self.managers.remove(&type_id);
                },
                _ => ()
            }
        }
    }

    pub(crate) async fn wait_until_stopped(&self) {
        WaitSystemStop::new(self.clone()).await;
    }

    pub(crate) fn create<A: Actor>(&self) -> ActorsManager<A> {
        // TOOD: Check if sending self and the Sender makes sense as the Sender is already in self.
        // Removing the sender here may allow to remove the system_director_report_loop.
        ActorsManager::<A>::new(self.clone())
    }

    pub(crate) fn stop_system(&self) {
        let mut handlers = vec!();
        for manager in self.managers.iter() {
            //TODO: This is wrong. The whole model is wrong as the end command should be message and the "remove myself" from the queue should be the sync part. 
            let manager = self.managers.remove(manager.key()).unwrap().1;
            handlers.push(task::spawn(async move {manager.end();}));
        }
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
