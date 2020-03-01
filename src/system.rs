use async_std::sync::Arc;
use crate::envelope::Envelope;
use crate::Actor;
use crate::actors_manager::ActorsManager;
use crate::Handle;
use dashmap::DashMap;
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use async_std::sync::Sender;

pub struct System {
    actor_managers: DashMap<TypeId, Box<dyn Any + Send>>,
    address_book: AddressBook,
}

impl System {
    pub fn new() -> System {
        System {
            actor_managers: DashMap::new(),
            address_book: AddressBook::new(),
        }
    }

    pub async fn send<A, M>(&self, id: A::Id, message: M)
    where
        A: Actor + Handle<M>,
        M: 'static + Send + Debug,
    {
        let type_id = TypeId::of::<A>();

        let mut manager = match self.actor_managers.get_mut(&type_id) {
            Some(manager) => manager,
            None => {
                self.add::<A>(id.clone()).await;
                match self.actor_managers.get_mut(&type_id) {
                    Some(manager) => manager,
                    None => unreachable!(),
                }
            }
        };

        match manager.downcast_mut::<ActorsManager<A>>() {
            Some(manager) => manager.send(id, message).await,
            None => unreachable!(),
        };
    }

    async fn add<A: Actor>(&self, id: A::Id) {
        let type_id = TypeId::of::<A>();

        let mut manager = ActorsManager::<A>::new(self.address_book.clone()).await;

        manager.add(id).await;

        self.actor_managers.insert(type_id, Box::new(manager));
    }
}

impl Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActeurSystem ()")
    }
}

#[derive(Debug)]
pub(crate) struct AddressBook {
    senders: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>
}

impl AddressBook {
    pub fn new() -> AddressBook {
        AddressBook {
            senders: Arc::new(DashMap::new()),
        }
    }

    pub async fn get<A>(&self) -> Sender<Box<dyn Envelope<Actor = A>>>
    where
        A: Actor
    {
        let type_id = TypeId::of::<A>();

        let mut sender = match self.senders.get_mut(&type_id) {
            Some(manager) => manager,
            //TODO: Implement the create Actor manager in the system
            None => unreachable!(),
        };

        match sender.downcast_mut::<Sender<Box<dyn Envelope<Actor = A>>>>() {
            Some(sender) => sender.clone(),
            None => unreachable!(),
        }
    }

    pub async fn add<A: Actor>(&self, sender: Sender<Box<dyn Envelope<Actor = A>>>) {
        let type_id = TypeId::of::<A>();

        self.senders.insert(type_id, Box::new(sender));
    }
}

impl Clone for AddressBook {
    fn clone(&self) -> Self {
        AddressBook {
            senders: self.senders.clone(),
        }
    }
}
