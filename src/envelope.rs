use crate::actor_proxy::ActorProxy;
use crate::{Actor, Assistant, Handle};
use async_trait::async_trait;
use std::fmt::Debug;
use std::marker::PhantomData;

#[async_trait]
pub(crate) trait Envelope: Send + Debug {
    type Actor: Actor;

    async fn dispatch(&mut self, actor: &mut Self::Actor, assistant: Assistant);
}

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

    pub async fn dispatch(&mut self, actor: &mut A, assistant: Assistant) {
        if let Some(message) = self.message.take() {
            <A as Handle<M>>::handle(actor, message, assistant).await;
        }
    }
}

#[async_trait]
impl<A: Actor + Handle<M>, M: Send + Debug> Envelope for Letter<A, M> {
    type Actor = A;

    async fn dispatch(&mut self, actor: &mut A, assistant: Assistant) {
        Letter::<A, M>::dispatch(self, actor, assistant).await
    }
}

pub(crate) trait ManagerEnvelope: Send + Debug {
    type Actor: Actor;

    fn dispatch(&mut self, manager: &mut ActorProxy<Self::Actor>);

    fn get_actor_id(&self) -> <<Self as ManagerEnvelope>::Actor as Actor>::Id;
}

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

    pub fn dispatch(&mut self, manager: &mut ActorProxy<A>) {
        if let Some(message) = self.message.take() {
            manager.send(message);
        }
    }
}

impl<A: Actor + Handle<M>, M: 'static + Send + Debug> ManagerEnvelope for ManagerLetter<A, M> {
    type Actor = A;

    fn dispatch(&mut self, manager: &mut ActorProxy<Self::Actor>) {
        ManagerLetter::<A, M>::dispatch(self, manager)
    }

    fn get_actor_id(&self) -> A::Id {
        ManagerLetter::<A, M>::get_actor_id(self)
    }
}
