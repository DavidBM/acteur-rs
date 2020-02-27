use crate::actor::Actor;
use crate::handle::Handle;
use async_trait::async_trait;
use std::fmt::Debug;
use std::marker::PhantomData;

#[async_trait]
pub(crate) trait Envelope: Send + Debug {
    type Actor: Actor;

    async fn dispatch(&mut self, actor: &mut Self::Actor);
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

    pub async fn dispatch(&mut self, actor: &mut A) {
        match self.message.take() {
            Some(message) => {
                <A as Handle<M>>::handle(actor, message).await;
            }
            None => (),
        };
    }
}

#[async_trait]
impl<A: Actor + Handle<M>, M: Send + Debug> Envelope for Letter<A, M> {
    type Actor = A;

    async fn dispatch(&mut self, actor: &mut A) {
        Letter::<A, M>::dispatch(self, actor).await
    }
}
