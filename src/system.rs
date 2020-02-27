use crate::Actor;
use crate::actors_manager::ActorsManager;
use crate::Handle;
use dashmap::DashMap;
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;

pub struct System {
    actor_managers: DashMap<TypeId, Box<dyn Any>>,
}

impl System {
    pub fn new() -> System {
        System {
            actor_managers: DashMap::new(),
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

        let mut manager = ActorsManager::<A>::new().await;

        manager.add(id).await;

        self.actor_managers.insert(type_id, Box::new(manager));
    }
}

impl Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActeurSystem ()")
    }
}

