use crate::actor::Actor;
use crate::actors_manager::{ActorManagerProxyCommand, ActorsManager, Manager};
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task::spawn,
};
use dashmap::DashMap;
use futures::task::AtomicWaker;
use std::any::{Any, TypeId};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) enum ActorManagerReport {
    ManagerEnded(TypeId),
}

#[derive(Debug)]
pub(crate) struct AddressBook {
    senders: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    waker_for_sopped_manager: Arc<AtomicWaker>,
    manager_report_sender: Sender<ActorManagerReport>,
}

impl AddressBook {
    pub(crate) fn new() -> AddressBook {
        let (sender, receiver) = channel::<ActorManagerReport>(1);

        let sender_list = Arc::new(DashMap::new());
        let manager_list = Arc::new(DashMap::new());
        let waker = Arc::new(AtomicWaker::new());

        let address_book = AddressBook {
            senders: sender_list.clone(),
            managers: manager_list.clone(),
            waker_for_sopped_manager: waker.clone(),
            manager_report_sender: sender,
        };

        address_book_report_loop(receiver, sender_list, manager_list, waker);

        address_book
    }

    pub(crate) fn get<A>(&self) -> Option<Sender<ActorManagerProxyCommand<A>>>
    where
        A: Actor,
    {
        let type_id = TypeId::of::<A>();

        let mut sender = match self.senders.get_mut(&type_id) {
            Some(manager) => manager,
            None => {
                // TODO: Check if the creation of new actors should be really here
                self.create::<A>();
                match self.senders.get_mut(&type_id) {
                    Some(manager) => manager,
                    None => unreachable!(),
                }
            }
        };

        match sender.downcast_mut::<Sender<ActorManagerProxyCommand<A>>>() {
            Some(sender) => Some(sender.clone()),
            None => unreachable!(),
        }
    }

    pub(crate) fn create<A: Actor>(&self) {
        let manager = ActorsManager::<A>::new(self.clone(), self.manager_report_sender.clone());
        let sender = manager.get_sender();

        let type_id = TypeId::of::<A>();

        self.senders.insert(type_id, Box::new(sender));
        self.managers.insert(type_id, Box::new(manager));
    }

    pub(crate) fn stop_all(&self) {
        for manager in self.managers.iter() {
            manager.end();
        }
    }

    pub(crate) fn count_actor_managers(&self) -> usize {
        self.managers.len()
    }
}

fn address_book_report_loop(
    receiver: Receiver<ActorManagerReport>,
    senders: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    waker: Arc<AtomicWaker>,
) {
    spawn(async move {
        while let Some(report) = receiver.recv().await {
            match report {
                ActorManagerReport::ManagerEnded(id) => {
                    senders.remove(&id);
                    managers.remove(&id);

                    match (senders.is_empty(), managers.is_empty()) {
                        (true, true) => waker.wake(),
                        (false, true) | (true, false) => unreachable!(),
                        (false, false) => (),
                    };
                }
            }
        }
    });
}

impl Clone for AddressBook {
    fn clone(&self) -> Self {
        AddressBook {
            senders: self.senders.clone(),
            managers: self.managers.clone(),
            waker_for_sopped_manager: self.waker_for_sopped_manager.clone(),
            manager_report_sender: self.manager_report_sender.clone(),
        }
    }
}

pub(crate) struct WaitSystemStop(AddressBook);

impl WaitSystemStop {
    pub fn new(system: AddressBook) -> WaitSystemStop {
        WaitSystemStop(system)
    }
}

impl Future for WaitSystemStop {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.0.count_actor_managers() > 0 {
            self.0.waker_for_sopped_manager.register(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
