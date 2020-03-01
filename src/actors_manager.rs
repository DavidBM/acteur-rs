use async_std::sync::Arc;
use async_std::task::spawn;
use async_std::sync::channel;
use async_std::sync::Sender;
use async_std::sync::Receiver;
use crate::envelope::ManagerEnvelope;
use crate::system::AddressBook;
use crate::actor::Actor;
use crate::actor_proxy::ActorProxy;
use dashmap::DashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub(crate) enum ActorManagerProxyCommand<A: Actor> {
    Dispatch(Box<dyn ManagerEnvelope<Actor = A>>),
    End,
}

#[derive(Debug)]
pub(crate) struct ActorsManager<A: Actor> {
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    sender: Sender<ActorManagerProxyCommand<A>>,
}

impl<A: Actor> ActorsManager<A> {
    pub fn new(address_book: AddressBook) -> ActorsManager<A> {
        let (sender, receiver): (Sender<ActorManagerProxyCommand<A>>, Receiver<ActorManagerProxyCommand<A>>) =
            channel(150_000);

        address_book.add(sender.clone());

        let actors = Arc::new(DashMap::new());

        {
            let actors = actors.clone();

            spawn(async move {
                while let Some(command) = receiver.recv().await {
                    //println!("Hey yo! There are {} actors alive!", actors.len());
                    match command {
                        ActorManagerProxyCommand::Dispatch(mut command) => {
                            let actor_id = command.get_actor_id();

                            if let Some(mut actor) = actors.get_mut(&actor_id) {
                                command.dispatch(&mut actor);
                                continue;
                            }

                            let actor = ActorProxy::<A>::new(address_book.clone(), actor_id.clone());
                            actors.insert(actor_id.clone(), actor);

                            match actors.get_mut(&actor_id) {
                                Some(mut actor) => {
                                    command.dispatch(&mut actor);
                                },
                                None => unreachable!(),
                            }
                        },  
                        ActorManagerProxyCommand::End => break,
                    }
                }
            });
        }

        ActorsManager {
            actors,
            sender
        }
    }

    /*pub async fn end(&self) {
        for actor in &self.actors {
            actor.end().await;
        }
    }*/
}

