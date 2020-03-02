use crate::actors_manager::ActorManagerProxyCommand;
use crate::envelope::{Envelope, Letter, ManagerLetter};
use crate::system::AddressBook;
use crate::{Actor, Handle};
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
    address_book: Secretary,
}

impl<A: Actor> ActorProxy<A> {
    pub fn new(address_book: AddressBook, id: A::Id) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let secretary = Secretary::new(address_book);

        {
            let secretary = secretary.clone();
            let sender = sender.clone();

            spawn(async move {
                let mut actor = A::activate(id).await;

                spawn(async move {
                    while let Some(command) = receiver.recv().await {
                        match command {
                            ActorProxyCommand::Dispatch(mut envelope) => {
                                envelope.dispatch(&mut actor, secretary.clone()).await
                            }
                            ActorProxyCommand::End => {
                                // If there are any message left, we postpone the shutdown.
                                if !receiver.is_empty() {
                                    sender.send(ActorProxyCommand::End).await;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                });
            });
        }

        ActorProxy {
            sender,
            address_book: secretary,
        }
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

pub struct Secretary {
    address_book: AddressBook,
}

impl Secretary {
    pub(crate) fn new(address_book: AddressBook) -> Secretary {
        Secretary { address_book }
    }

    pub async fn send<A: Actor + Handle<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        if let Some(sender) = self.address_book.get::<A>() {
            sender
                .send(ActorManagerProxyCommand::Dispatch(Box::new(
                    ManagerLetter::<A, M>::new(actor_id, message),
                )))
                .await;
        }
    }

    pub async fn stop_system(&self) {
        self.address_book.stop_all();
    }
}

impl Clone for Secretary {
    fn clone(&self) -> Self {
        Secretary {
            address_book: self.address_book.clone(),
        }
    }
}

impl Debug for Secretary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActorSecretary ()")
    }
}
