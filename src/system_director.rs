use crate::actor_proxy::ActorReport;
use crate::actors_manager::{ActorManagerProxyCommand, ActorsManager, Manager};
use crate::envelope::ManagerLetter;
use crate::{Actor, Handle};
use crate::actors_lifecycle_director::ActorsLifecycleDirector;
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task,
};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use futures::task::AtomicWaker;
use std::{
    any::TypeId,
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub(crate) enum ActorManagerReport {
    ManagerEnded(TypeId),
}

// TODO: This structure is getting big and with several responsiblities, maybe it should be splitted.
#[derive(Debug)]
pub(crate) struct SystemDirector {
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>, // DELETE ?
    manager_report_sender: Sender<ActorManagerReport>,
    // TODO: Should be a WakerSet as there may be more than one thread that wants to wait
    waker: Arc<AtomicWaker>,
    lifecycle_director: Arc<Option<ActorsLifecycleDirector>>
}

impl SystemDirector {
    pub(crate) fn new() -> SystemDirector {
        let (sender, receiver) = channel::<ActorManagerReport>(1);

        let manager_list = Arc::new(DashMap::new());
        let waker = Arc::new(AtomicWaker::new());

        let mut system_director = SystemDirector {
            managers: manager_list.clone(),
            manager_report_sender: sender,
            waker: waker.clone(),
            lifecycle_director: Arc::new(None),
        };

        let lifecycle_director = ActorsLifecycleDirector::new(system_director.clone(), 300);

        *Arc::make_mut(&mut system_director.lifecycle_director) = Some(lifecycle_director);

        task::spawn(system_director_report_loop(receiver, manager_list, waker));

        system_director
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
        // Removing the sender here may allow to remove the system_director_report_loop.
        ActorsManager::<A>::new(self.clone(), self.manager_report_sender.clone())
    }

    pub(crate) fn stop_system(&self) {
        for manager in self.managers.iter() {
            manager.end();
        }

        if let Some(director) = self.lifecycle_director.as_ref() {
            director.stop();
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

    pub(crate) fn end_filtered(&self, filter_fn: Box<dyn Fn(ActorReport) -> bool>) {
        let filter_fn = Box::new(filter_fn);
        for manager in self.managers.iter() {
            manager.end_filtered(&filter_fn);
        }
    }
}

async fn system_director_report_loop(
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
            lifecycle_director: self.lifecycle_director.clone(),
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
