use crate::system::AddressBook;
use crate::envelope::Envelope;
use crate::handle::Handle;
use crate::envelope::Letter;
use async_std::sync::channel;
use async_std::sync::Receiver;
use async_std::sync::Sender;
use async_std::task::spawn;
use std::fmt::Debug;

use crate::actor::Actor;

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
    pub async fn new(address_book: AddressBook, id: A::Id) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let mut actor = A::activate(id).await;

        let secretary = Secretary::new(address_book);

        {
            let secretary = secretary.clone();
            spawn(async move {
                while let Some(command) = receiver.recv().await {
                    match command {
                        ActorProxyCommand::End => break,
                        ActorProxyCommand::Dispatch(mut envelope) => {
                            envelope.dispatch(&mut actor, secretary.clone()).await
                        }
                    }
                }
            });
        }

        ActorProxy { sender, address_book: secretary }
    }

    pub async fn send<M: 'static>(&self, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {
        let message = Letter::<A, M>::new(message);

        // TODO: Handle the channel disconnection properly
        self.sender
            .send(ActorProxyCommand::Dispatch(Box::new(message)))
            .await;
    }

    /*pub async fn end(&self) {
        self.sender.send(ActorProxyCommand::End).await;
    }*/
}

pub struct Secretary {
    address_book: AddressBook,
}

impl Secretary {
    pub(crate) fn new(address_book: AddressBook) -> Secretary {
        Secretary {
            address_book,
        }
    }

    pub async fn send<A: Actor + Handle<M>, M: Debug + Send + 'static>(&self, message: M) {
        let sender = self.address_book.get::<A>().await;
        sender.send(Box::new(Letter::<A, M>::new(message))).await;
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
