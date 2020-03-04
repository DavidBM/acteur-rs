use crate::actor_proxy::ActorProxy;
use crate::envelope::ManagerEnvelope;
use crate::system::AddressBook;
use crate::Actor;
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task::spawn,
};
use dashmap::DashMap;
use std::fmt::Debug;

pub(crate) trait Manager: Send + Sync + Debug {
    fn end(&self);
}

#[derive(Debug)]
pub(crate) enum ActorManagerProxyCommand<A: Actor> {
    Dispatch(Box<dyn ManagerEnvelope<Actor = A>>),
    End,
}

#[derive(Debug)]
pub(crate) enum ActorProxyReport<A: Actor> {
    ActorStopped(A::Id),
    AllActorsStopped,
}

#[derive(Debug)]
pub(crate) struct ActorsManager<A: Actor> {
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    sender: Sender<ActorManagerProxyCommand<A>>,
    report_sender: Sender<ActorProxyReport<A>>,
}

impl<A: Actor> ActorsManager<A> {
    pub fn new(address_book: AddressBook) -> ActorsManager<A> {
        let (sender, receiver) = channel::<ActorManagerProxyCommand<A>>(150_000);

        let (report_sender, report_receiver) = channel::<ActorProxyReport<A>>(1);

        let actors = Arc::new(DashMap::new());

        actor_manager_loop(
            receiver,
            sender.clone(),
            actors.clone(),
            address_book,
            report_sender.clone(),
        );

        actor_proxy_report_loop(report_receiver, actors.clone());

        ActorsManager {
            actors,
            sender,
            report_sender,
        }
    }

    pub(crate) fn end(&self) {
        let sender = self.sender.clone();
        spawn(async move {
            sender.send(ActorManagerProxyCommand::End).await;
        });
    }

    pub(crate) fn get_sender(&self) -> Sender<ActorManagerProxyCommand<A>> {
        self.sender.clone()
    }

    /*pub(crate) fn count(&self) -> usize {W
        self.actors.len()
    }*/
}

fn actor_manager_loop<A: Actor>(
    receiver: Receiver<ActorManagerProxyCommand<A>>,
    sender: Sender<ActorManagerProxyCommand<A>>,
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    address_book: AddressBook,
    report_sender: Sender<ActorProxyReport<A>>,
) {
    spawn(async move {
        while let Some(command) = receiver.recv().await {
            match command {
                ActorManagerProxyCommand::Dispatch(mut command) => {
                    let actor_id = command.get_actor_id();

                    if let Some(mut actor) = actors.get_mut(&actor_id) {
                        command.dispatch(&mut actor);
                        continue;
                    }

                    let actor = ActorProxy::<A>::new(
                        address_book.clone(),
                        actor_id.clone(),
                        report_sender.clone(),
                    );

                    actors.insert(actor_id.clone(), actor);

                    match actors.get_mut(&actor_id) {
                        Some(mut actor) => {
                            command.dispatch(&mut actor);
                        }
                        None => unreachable!(),
                    }
                }
                ActorManagerProxyCommand::End => {
                    // If there are any message left, we postpone the shutdown.
                    if !receiver.is_empty() {
                        // TODO: Check if the actor is already scheduled to stop in order to avoid
                        // the case where 2 End mesages are in the channel
                        sender.send(ActorManagerProxyCommand::End).await;
                    } else {
                        for actor in actors.iter() {
                            actor.end().await;
                        }

                        report_sender.send(ActorProxyReport::AllActorsStopped).await;
                        break;
                    }
                }
            }
        }
    });
}

fn actor_proxy_report_loop<A: Actor>(
    receiver: Receiver<ActorProxyReport<A>>,
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
) {
    spawn(async move {
        while let Some(command) = receiver.recv().await {
            match command {
                ActorProxyReport::ActorStopped(id) => {
                    actors.remove(&id);
                }
                ActorProxyReport::AllActorsStopped => break,
            }
        }
    });
}

impl<A: Actor> Clone for ActorsManager<A> {
    fn clone(&self) -> Self {
        ActorsManager {
            actors: self.actors.clone(),
            sender: self.sender.clone(),
            report_sender: self.report_sender.clone(),
        }
    }
}

impl<A: Actor> Manager for ActorsManager<A> {
    fn end(&self) {
        ActorsManager::<A>::end(self);
    }
}
