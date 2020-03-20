use crate::actor_proxy::ActorReport;
use crate::actors_manager::{ActorManagerProxyCommand, ActorsManager, Manager};
use crate::envelope::ManagerLetter;
use crate::{Actor, Handle};
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task,
};
use dashmap::DashMap;
use futures::task::AtomicWaker;
use std::{
    any::TypeId,
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use dashmap::mapref::entry::Entry;

pub(crate) enum ActorManagerReport {
    ManagerEnded(TypeId),
}

// TODO: This structure is getting big and with several responsiblities, maybe it should be splitted.
#[derive(Debug)]
pub(crate) struct SystemDirector {
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,  // DELETE ?
    manager_report_sender: Sender<ActorManagerReport>,
    // TODO: Should be a WakerSet as there may be more than one thread that wants to wait
    waker: Arc<AtomicWaker>,
}

impl SystemDirector {
    pub(crate) fn new() -> SystemDirector {
        let (sender, receiver) = channel::<ActorManagerReport>(1);

        let manager_list = Arc::new(DashMap::new());
        let waker = Arc::new(AtomicWaker::new());

        let address_book = SystemDirector {
            managers: manager_list.clone(),
            manager_report_sender: sender,
            waker: waker.clone(),
        };

        task::spawn(address_book_report_loop(
            receiver,
            manager_list,
            waker,
        ));

        address_book
    }

    // Ensures that there is a manager for that type and returns a sender to it
    fn get_or_create_manager_sender<A: Actor>(&self) -> Sender<ActorManagerProxyCommand<A>> {
        let type_id = TypeId::of::<A>();

        let managers_entry = self.managers.entry(type_id);

        let ref mut manager = match managers_entry {
            Entry::Occupied(entry) => entry.into_ref(),
            Entry::Vacant(entry) => {
                let manager = self.create::<A>();
                entry.insert(Box::new(manager))
            }
        };

        match manager.get_sender_as_any().downcast_mut::<Sender<ActorManagerProxyCommand<A>>>() {
            Some(sender) => sender.clone(),
            // If type is not matching, crash as  we don't really want to
            // run the framework with a bug like this
            None => unreachable!(),
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

    pub(crate) fn create<A: Actor>(&self) -> ActorsManager<A> {
        // TOOD: Check if sending self and the Sender makes sense as the Sender is already in self.
        // Removing the sender here may allow to remove the address_book_report_loop.
        ActorsManager::<A>::new(self.clone(), self.manager_report_sender.clone())
    }

    pub(crate) fn stop_all(&self) {
        for manager in self.managers.iter() {
            manager.end();
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

async fn address_book_report_loop(
    receiver: Receiver<ActorManagerReport>,
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    waker: Arc<AtomicWaker>,
) {
    while let Some(report) = receiver.recv().await {
        match report {
            ActorManagerReport::ManagerEnded(id) => {
                managers.remove(&id);

                if managers.is_empty() {
                    waker.wake();
                }
            }
        }
    }
}

impl Clone for SystemDirector {
    fn clone(&self) -> Self {
        SystemDirector {
            managers: self.managers.clone(),
            manager_report_sender: self.manager_report_sender.clone(),
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
