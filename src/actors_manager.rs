use crate::actor_proxy::ActorProxy;
use crate::address_book::{ActorManagerReport, AddressBook};
use crate::envelope::ManagerEnvelope;
use crate::Actor;
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task::spawn,
};
use dashmap::DashMap;
use std::any::TypeId;
use std::fmt::Debug;

pub(crate) trait Manager: Send + Sync + Debug {
    fn end(&self);
}

#[derive(Debug)]
pub(crate) enum ActorManagerProxyCommand<A: Actor> {
    Dispatch(Box<dyn ManagerEnvelope<Actor = A>>),
    EndActor(A::Id),
    End,
}

#[derive(Debug)]
pub(crate) enum ActorProxyReport<A: Actor> {
    ActorStopped(A::Id),
}

#[derive(Debug)]
pub(crate) struct ActorsManager<A: Actor> {
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    sender: Sender<ActorManagerProxyCommand<A>>,
    report_sender: Sender<ActorProxyReport<A>>,
}

impl<A: Actor> ActorsManager<A> {
    pub fn new(
        address_book: AddressBook,
        manager_report_sender: Sender<ActorManagerReport>,
    ) -> ActorsManager<A> {
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

        actor_proxy_report_loop(
            report_receiver,
            actors.clone(),
            manager_report_sender.clone(),
        );

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
                ActorManagerProxyCommand::Dispatch(command) => {
                    process_dispatch_command(command, &actors, &address_book, &report_sender);
                }
                ActorManagerProxyCommand::EndActor(id) => {
                    if let Some(actor) = actors.get_mut(&id) {
                        actor.end().await;
                        return;
                    }
                }
                ActorManagerProxyCommand::End => {
                    // We may find cases where we can have several End command in a row. In that case,
                    // we want to consume all the end command together until we find nothing or a not-end command
                    match recv_until_not_end_command(receiver.clone()).await {
                        None => {
                            for actor in actors.iter() {
                                actor.end().await;
                            }
                            break;
                        }
                        Some(ActorManagerProxyCommand::Dispatch(command)) => {
                            // If there are any message left, we postpone the shutdown.
                            sender.send(ActorManagerProxyCommand::End).await;
                            process_dispatch_command(
                                command,
                                &actors,
                                &address_book,
                                &report_sender,
                            );
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    });
}

fn process_dispatch_command<A: Actor>(
    mut command: Box<dyn ManagerEnvelope<Actor = A>>,
    actors: &Arc<DashMap<A::Id, ActorProxy<A>>>,
    address_book: &AddressBook,
    report_sender: &Sender<ActorProxyReport<A>>,
) {
    let actor_id = command.get_actor_id();

    if let Some(mut actor) = actors.get_mut(&actor_id) {
        command.dispatch(&mut actor);
        return;
    }

    let mut actor = ActorProxy::<A>::new(
        address_book.clone(),
        actor_id.clone(),
        report_sender.clone(),
    );

    command.dispatch(&mut actor);

    actors.insert(actor_id, actor);
}

fn actor_proxy_report_loop<A: Actor>(
    receiver: Receiver<ActorProxyReport<A>>,
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    system_report: Sender<ActorManagerReport>,
) {
    spawn(async move {
        while let Some(command) = receiver.recv().await {
            match command {
                ActorProxyReport::ActorStopped(id) => {
                    actors.remove(&id);
                    //println!("Actor stopping... {} remaining", actors.len());
                    if actors.is_empty() {
                        system_report
                            .send(ActorManagerReport::ManagerEnded(TypeId::of::<A>()))
                            .await;
                        break;
                    }
                }
            }
        }
    });
}

async fn recv_until_not_end_command<A: Actor>(
    receiver: Receiver<ActorManagerProxyCommand<A>>,
) -> Option<ActorManagerProxyCommand<A>> {
    if receiver.is_empty() {
        return None;
    }

    while let Some(command) = receiver.recv().await {
        match command {
            ActorManagerProxyCommand::End => {
                if receiver.is_empty() {
                    return None;
                } else {
                    continue;
                }
            }
            _ => return Some(command),
        }
    }

    None
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
