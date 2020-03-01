use crate::actor_proxy::ActorProxy;
use crate::actor::Actor;
use crate::actor_proxy::Secretary;
use crate::handle::Handle;
use async_trait::async_trait;
use std::fmt::Debug;
use std::marker::PhantomData;

#[async_trait]
pub(crate) trait Envelope: Send + Debug {
    type Actor: Actor;

    async fn dispatch(&mut self, actor: &mut Self::Actor, secretary: Secretary);
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

    pub async fn dispatch(&mut self, actor: &mut A, secretary: Secretary) {
        match self.message.take() {
            Some(message) => {
                <A as Handle<M>>::handle(actor, message, secretary).await;
            }
            None => (),
        };
    }
}

#[async_trait]
impl<A: Actor + Handle<M>, M: Send + Debug> Envelope for Letter<A, M> {
    type Actor = A;

    async fn dispatch(&mut self, actor: &mut A, secretary: Secretary) {
        Letter::<A, M>::dispatch(self, actor, secretary).await
    }
}

pub(crate) trait ManagerEnvelope: Send + Debug {
    type Actor: Actor;

    fn dispatch(
        &mut self,
        manager: &mut ActorProxy<Self::Actor>,
    );

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
        match self.message.take() {
            Some(message) => {
                manager.send(message);
            }
            None => (),
        };
    }
}

impl<A: Actor + Handle<M>, M: 'static + Send + Debug> ManagerEnvelope for ManagerLetter<A, M> {
    type Actor = A;

    fn dispatch(
        &mut self,
        manager: &mut ActorProxy<Self::Actor>,
    ) {
        ManagerLetter::<A, M>::dispatch(self, manager)
    }

    fn get_actor_id(&self) -> A::Id {
        ManagerLetter::<A, M>::get_actor_id(self)
    }
}
