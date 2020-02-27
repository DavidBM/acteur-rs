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
}

impl<A: Actor> ActorProxy<A> {
    pub async fn new(id: A::Id) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let mut actor = A::activate(id).await;

        spawn(async move {
            while let Some(command) = receiver.recv().await {
                match command {
                    ActorProxyCommand::End => break,
                    ActorProxyCommand::Dispatch(mut envelope) => {
                        envelope.dispatch(&mut actor).await
                    }
                }
            }
        });

        ActorProxy { sender }
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

    pub async fn end(&self) {
        self.sender.send(ActorProxyCommand::End).await;
    }
}
