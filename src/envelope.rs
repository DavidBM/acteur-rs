use crate::actor_proxy::ActorProxy;
use crate::{Actor, Assistant, Handle};
use async_trait::async_trait;
use std::fmt::Debug;
use std::marker::PhantomData;

/// This structures encapsulate the message and capture the Type through a PhantonType,
/// making them safe to send via channels that may allow different types Letter<A> != Letter<B>
/// at the same time that you are able to use the type later in the dispatch/deliver method.
/// The way this works is that the Letter/Envelope will receive the Actor and it will be the
/// one calling the method of the Actor A or B.

/// There are two variants of these structures. One is the Envelope/Letter pair, with executes the
/// handle method from the Actor trait directly. The other is the ManagerEnvelope/ManagerLetter that
/// sends the message to the ActorProxy inbox channel. The only difference between them both beyond
/// the difference in the method calling is that the former contains the Actor::Id that should
/// receive the message.

/// Trait that represents a message directed to an actor instance.
#[async_trait]
pub(crate) trait Envelope: Send + Debug {
    type Actor: Actor;

    async fn dispatch(&mut self, actor: &mut Self::Actor, assistant: &Assistant<Self::Actor>);
}

/// This struct implements `Envelope` and stores the message and the Actors type. This is
/// required as we are sending the message through a channel that contain Boxed Envelope Traits
/// which cannot contain the M type as they are generic for all messages.
#[derive(Debug)]
pub(crate) struct Letter<A: Actor, M: Debug> {
    message: Option<M>,
    phantom: PhantomData<A>,
}

impl<A: Handle<M> + Actor, M: Debug> Letter<A, M> {
    pub fn new(message: M) -> Self
    where
        A: Handle<M>,
    {
        Letter {
            message: Some(message),
            phantom: PhantomData,
        }
    }

    pub async fn dispatch(&mut self, actor: &mut A, assistant: &Assistant<A>) {
        if let Some(message) = self.message.take() {
            <A as Handle<M>>::handle(actor, message, assistant).await;
        }
    }
}

#[async_trait]
impl<A: Actor + Handle<M>, M: Send + Debug> Envelope for Letter<A, M> {
    type Actor = A;

    async fn dispatch(&mut self, actor: &mut A, assistant: &Assistant<A>) {
        Letter::<A, M>::dispatch(self, actor, assistant).await
    }
}

/// Same as Envelope but for Actors Managers. Actors Managers control group of actors of the same type.
/// This especial envelope assumes that there is an Actor::Id and an ActorProxy
#[async_trait]
pub(crate) trait ManagerEnvelope: Send + Debug {
    type Actor: Actor;

    async fn deliver(&mut self, manager: &mut ActorProxy<Self::Actor>);

    fn get_actor_id(&self) -> <<Self as ManagerEnvelope>::Actor as Actor>::Id;
}

/// The struct that implements `ManagerEnvelope`. Same as Letter, but with the Actor::Id in it in order to route the message
#[derive(Debug)]
pub(crate) struct ManagerLetter<A: Actor, M: Debug> {
    message: Option<M>,
    actor_id: A::Id,
    phantom: PhantomData<A>,
}

impl<A: Handle<M> + Actor, M: 'static + Send + Debug> ManagerLetter<A, M> {
    pub fn new(actor_id: A::Id, message: M) -> Self
    where
        A: Handle<M>,
    {
        ManagerLetter {
            message: Some(message),
            actor_id,
            phantom: PhantomData,
        }
    }

    pub fn get_actor_id(&self) -> A::Id {
        self.actor_id.clone()
    }

    pub async fn deliver(&mut self, manager: &mut ActorProxy<A>) {
        if let Some(message) = self.message.take() {
            manager.send(message).await;
        }
    }
}

#[async_trait]
impl<A: Actor + Handle<M>, M: 'static + Send + Debug> ManagerEnvelope for ManagerLetter<A, M> {
    type Actor = A;

    async fn deliver(&mut self, manager: &mut ActorProxy<Self::Actor>) {
        ManagerLetter::<A, M>::deliver(self, manager).await
    }

    fn get_actor_id(&self) -> A::Id {
        ManagerLetter::<A, M>::get_actor_id(self)
    }
}
