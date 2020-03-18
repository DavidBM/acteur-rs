use crate::actors_manager::ActorProxyReport;
use crate::address_book::AddressBook;
use crate::envelope::{Envelope, Letter};
use crate::{Actor, Assistant, Handle};
use async_std::{
    sync::{channel, Receiver, Sender},
    task::spawn,
};
use std::fmt::Debug;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) enum ActorProxyCommand<A: Actor> {
    Dispatch(Box<dyn Envelope<Actor = A>>),
    End,
}

#[derive(Debug)]
pub(crate) struct ActorProxy<A: Actor> {
    sender: Sender<ActorProxyCommand<A>>,
    last_sent_message_time: SystemTime,
}

impl<A: Actor> ActorProxy<A> {
    pub fn new(
        address_book: AddressBook,
        id: A::Id,
        report_sender: Sender<ActorProxyReport<A>>,
    ) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let assistant = Assistant::new(address_book, id.clone());

        actor_loop(
            id,
            sender.clone(),
            receiver,
            assistant.clone(),
            report_sender,
        );

        ActorProxy {
            sender,
            last_sent_message_time: SystemTime::now(),
        }
    }

    pub async fn send<M: 'static>(&mut self, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {
        self.last_sent_message_time = SystemTime::now();

        let message = Letter::<A, M>::new(message);

        // TODO: Handle the channel disconnection properly
        self.sender
            .send(ActorProxyCommand::Dispatch(Box::new(message)))
            .await;
    }

    pub fn get_last_sent_message_time(&self) -> SystemTime {
        self.last_sent_message_time
    }

    pub async fn end(&self) {
        self.sender.send(ActorProxyCommand::End).await;
    }
}

fn actor_loop<A: Actor>(
    id: A::Id,
    sender: Sender<ActorProxyCommand<A>>,
    receiver: Receiver<ActorProxyCommand<A>>,
    assistant: Assistant<A>,
    report_sender: Sender<ActorProxyReport<A>>,
) {
    spawn(async move {
        let mut actor = A::activate(id.clone()).await;

        spawn(async move {
            while let Some(command) = receiver.recv().await {
                match command {
                    ActorProxyCommand::Dispatch(mut envelope) => {
                        envelope.dispatch(&mut actor, &assistant).await
                    }
                    ActorProxyCommand::End => {
                        // We may find cases where we can have several End command in a row. In that case,
                        // we want to consume all the end command together until we find nothing or a not-end command
                        match recv_until_not_end_command(receiver.clone()).await {
                            None => {
                                actor.deactivate().await;
                                report_sender.send(ActorProxyReport::ActorStopped(id)).await;
                                break;
                            }
                            Some(ActorProxyCommand::Dispatch(mut envelope)) => {
                                // If there are any message left, we postpone the shutdown.
                                sender.send(ActorProxyCommand::End).await;
                                envelope.dispatch(&mut actor, &assistant).await
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        });
    });
}

async fn recv_until_not_end_command<A: Actor>(
    receiver: Receiver<ActorProxyCommand<A>>,
) -> Option<ActorProxyCommand<A>> {
    if receiver.is_empty() {
        return None;
    }

    while let Some(command) = receiver.recv().await {
        match command {
            ActorProxyCommand::Dispatch(_) => return Some(command),
            ActorProxyCommand::End => {
                if receiver.is_empty() {
                    return None;
                } else {
                    continue;
                }
            }
        }
    }

    None
}
