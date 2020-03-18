use crate::actors_manager::{ActorManagerProxyCommand, ActorsManager, Manager};
use crate::envelope::ManagerLetter;
use crate::{Actor, Handle};
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task,
};
use dashmap::DashMap;
use futures::task::AtomicWaker;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) enum ActorManagerReport {
    ManagerEnded(TypeId),
}

// TODO: This structure is getting big and with several responsiblities, maybe it should be splitted.
#[derive(Debug)]
pub(crate) struct AddressBook {
    senders: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    manager_report_sender: Sender<ActorManagerReport>,
    // TODO: Should be a WakerSet as there may be more than one thread that wants to wait
    waker: Arc<AtomicWaker>,
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
            manager_report_sender: sender,
            waker: waker.clone(),
        };

        task::spawn(address_book_report_loop(
            receiver,
            sender_list,
            manager_list,
            waker,
        ));

        address_book
    }

    pub(crate) fn get_sender<A>(&self) -> Option<Sender<ActorManagerProxyCommand<A>>>
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

    pub async fn send<A: Actor + Handle<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        if let Some(sender) = self.get_sender::<A>() {
            sender
                .send(ActorManagerProxyCommand::Dispatch(Box::new(
                    ManagerLetter::new(actor_id, message),
                )))
                .await;
        }
    }

    pub(crate) async fn wait_until_stopped(&self) {
        WaitSystemStop::new(self.clone()).await;
    }

    pub(crate) fn create<A: Actor>(&self) {
        // TOOD: Check if sending self and the Sender makes sense as the Sender is already in self.
        // Removing the sender here may allow to remove the address_book_report_loop.
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

async fn address_book_report_loop(
    receiver: Receiver<ActorManagerReport>,
    senders: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    managers: Arc<DashMap<TypeId, Box<dyn Manager>>>,
    waker: Arc<AtomicWaker>,
) {
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
}

impl Clone for AddressBook {
    fn clone(&self) -> Self {
        AddressBook {
            senders: self.senders.clone(),
            managers: self.managers.clone(),
            manager_report_sender: self.manager_report_sender.clone(),
            waker: self.waker.clone(),
        }
    }
}

pub(crate) struct WaitSystemStop(AddressBook);

impl WaitSystemStop {
    pub fn new(waker: AddressBook) -> WaitSystemStop {
        WaitSystemStop(waker)
    }
}

impl Future for WaitSystemStop {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.0.count_actor_managers() > 0 {
            self.0.waker.register(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
