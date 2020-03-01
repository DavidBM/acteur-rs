use std::marker::PhantomData;
use async_std::task::spawn;
use crate::handle::Handle;
use crate::actors_manager::ActorManagerProxyCommand;
use async_std::sync::Arc;
use crate::actors_manager::ActorsManager;
use crate::envelope::ManagerLetter;
use crate::Actor;
use dashmap::DashMap;
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use async_std::sync::Sender;

pub struct System {
    address_book: AddressBook,
}

impl System {
    pub fn new() -> System {
        let address_book = AddressBook::new();
        System { address_book }
    }

    pub fn send<A: Actor + Handle<M>, M: Debug + Send + 'static>(&self, actor_id: A::Id, message: M) {
        match self.address_book.get::<A>() {
            Some(sender) => {
                spawn(async move {
                    sender.send(ActorManagerProxyCommand::Dispatch(Box::new(ManagerLetter::new(actor_id, message)))).await;
                });
            },
            None => {}
        }
    }

    pub fn block(&self) {
        async_std::task::block_on(async {
            async_std::future::pending::<()>().await;
        });
    }
}

impl Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActeurSystem ()")
    }
}

impl Clone for System {
    fn clone(&self) -> Self {
        System {
            address_book: self.address_book.clone(),
        }
    }
}


#[derive(Debug)]
pub(crate) struct AddressBook {
    senders: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl AddressBook {
    pub fn new() -> AddressBook {
        AddressBook {
            senders: Arc::new(DashMap::new()),
        }
    }

    pub fn get<A>(&self) -> Option<Sender<ActorManagerProxyCommand<A>>>
    where
        A: Actor
    {
        let type_id = TypeId::of::<A>();

        let mut sender = match self.senders.get_mut(&type_id) {
            Some(manager) => manager,
            None => {
                self.create::<A>();
                match self.senders.get_mut(&type_id) {
                    Some(manager) => manager,
                    None => unreachable!(),
                }
            },
        };

        match sender.downcast_mut::<Sender<ActorManagerProxyCommand<A>>>() {
            Some(sender) => Some(sender.clone()),
            None => unreachable!(),
        }
    }

    pub fn add<A: Actor>(&self, sender: Sender<ActorManagerProxyCommand<A>>) {
        let type_id = TypeId::of::<A>();

        self.senders.insert(type_id, Box::new(sender));
    }

    pub fn create<A: Actor>(&self) {
        ActorsManager::<A>::new(self.clone());
    }

    pub fn stop_all(&self) {
        for _sender in self.senders.iter() {
            //TODO: Find the way to send a typed message to all ActorManagers
            // Provably once the async_std channels can have try_recv (as it will allow
            // to have two queues, one for messages, other for special commands)
            //sender.send(ActorManagerProxyCommand::End);
        }
    }
}

impl Clone for AddressBook {
    fn clone(&self) -> Self {
        AddressBook {
            senders: self.senders.clone(),
        }
    }
}

#[derive(Debug)]
struct ManagerRepresentant<A: Actor> {
    phantom: PhantomData<A>
}
