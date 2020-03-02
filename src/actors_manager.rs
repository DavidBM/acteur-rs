use async_std::task::spawn;
use async_std::sync::{Arc, channel, Sender, Receiver};
use crate::envelope::ManagerEnvelope;
use crate::system::AddressBook;
use crate::Actor;
use crate::actor_proxy::ActorProxy;
use dashmap::DashMap;
use std::fmt::Debug;

pub(crate) trait Manager: Send + Sync + Debug {
    fn end(&self);
}

#[derive(Debug)]
pub(crate) enum ActorManagerProxyCommand<A: Actor> {
    Dispatch(Box<dyn ManagerEnvelope<Actor = A>>),
    End
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
            let sender = sender.clone();

            spawn(async move {
                loop {
                    match receiver.recv().await{
                        Some(command) => {
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
                                ActorManagerProxyCommand::End => {
                                    // If there are any message left, we postpone the shutdown.
                                    if receiver.len() > 0 {
                                        sender.send(ActorManagerProxyCommand::End).await;
                                    } else {
                                        for actor in actors.iter() {
                                            actor.end().await;
                                        }
                                        break
                                    }
                                },
                            }
                        },
                        _ => ()
                    }
                }
            });
        }

        ActorsManager {
            actors,
            sender,
        }
    }

    pub fn end(&self) {
        let sender = self.sender.clone();
        spawn(async move {
            sender.send(ActorManagerProxyCommand::End).await;
        });
    }
}

impl <A: Actor>Clone for ActorsManager<A> {
    fn clone(&self) -> Self {
        ActorsManager {
            actors: self.actors.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl <A: Actor>Manager for ActorsManager<A> {
    fn end(&self) {
        ActorsManager::<A>::end(self);
    }
}
