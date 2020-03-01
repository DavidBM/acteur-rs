use crate::system::AddressBook;
use crate::actor::Actor;
use crate::actor_proxy::ActorProxy;
use crate::handle::Handle;
use dashmap::DashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub(crate) struct ActorsManager<A: Actor> {
    actors: DashMap<A::Id, ActorProxy<A>>,
    address_book: AddressBook,
}

impl<A: Actor> ActorsManager<A> {
    pub async fn new(directory: AddressBook) -> ActorsManager<A> {
        ActorsManager {
            actors: DashMap::new(),
            address_book: directory,
        }
    }

    pub async fn add(&mut self, id: A::Id) {
        match self.actors.get_mut(&id) {
            Some(_) => (),
            None => {
                let actor = ActorProxy::<A>::new(self.address_book.clone(), id.clone()).await;
                self.actors.insert(id, actor);
            }
        }
    }

    pub async fn send<M: 'static>(&mut self, id: A::Id, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {
        if let Some(actor) = self.actors.get_mut(&id) {
            actor.send(message).await;
            return;
        }

        self.add(id.clone()).await;

        match self.actors.get_mut(&id) {
            Some(actor) => actor.send(message).await,
            None => unreachable!(),
        }
    }

    /*pub async fn end(&self) {
        for actor in &self.actors {
            actor.end().await;
        }
    }*/
}

