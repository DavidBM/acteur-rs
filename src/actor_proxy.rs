use crate::actors_manager::ActorProxyReport;
use crate::envelope::{Envelope, Letter};
use crate::system::AddressBook;
use crate::{Actor, Assistant, Handle};
use async_std::{
    sync::{channel, Receiver, Sender},
    task::spawn,
};
use std::fmt::Debug;

#[derive(Debug)]
enum ActorProxyCommand<A: Actor> {
    Dispatch(Box<dyn Envelope<Actor = A>>),
    End,
}

#[derive(Debug)]
pub(crate) struct ActorProxy<A: Actor> {
    sender: Sender<ActorProxyCommand<A>>,
    assistant: Assistant,
}

impl<A: Actor> ActorProxy<A> {
    pub fn new(
        address_book: AddressBook,
        id: A::Id,
        report_sender: Sender<ActorProxyReport<A>>,
    ) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let assistant = Assistant::new(address_book);

        actor_loop(
            id,
            sender.clone(),
            receiver,
            assistant.clone(),
            report_sender,
        );

        ActorProxy { sender, assistant }
    }

    pub fn send<M: 'static>(&self, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {
        let message = Letter::<A, M>::new(message);

        let sender = self.sender.clone();

        // TODO: Handle the channel disconnection properly
        spawn(async move {
            sender
                .send(ActorProxyCommand::Dispatch(Box::new(message)))
                .await;
        });
    }

    pub async fn end(&self) {
        self.sender.send(ActorProxyCommand::End).await;
    }
}

fn actor_loop<A: Actor>(
    id: A::Id,
    sender: Sender<ActorProxyCommand<A>>,
    receiver: Receiver<ActorProxyCommand<A>>,
    assistant: Assistant,
    report_sender: Sender<ActorProxyReport<A>>,
) {
    spawn(async move {
        let mut actor = A::activate(id.clone()).await;

        spawn(async move {
            while let Some(command) = receiver.recv().await {
                match command {
                    ActorProxyCommand::Dispatch(mut envelope) => {
                        envelope.dispatch(&mut actor, assistant.clone()).await
                    }
                    ActorProxyCommand::End => {
                        // If there are any message left, we postpone the shutdown.
                        if !sender.is_empty() {
                            // TODO: Check if the actor is already scheduled to stop in order to avoid
                            // the case where 2 End mesages are in the channel
                            sender.send(ActorProxyCommand::End).await;
                        } else {
                            actor.deactivate().await;
                            report_sender.send(ActorProxyReport::ActorStopped(id)).await;
                            break;
                        }
                    }
                }
            }
        });
    });
}
