use crate::actors::envelope::Letter;
use crate::services::handle::Notify;
use crate::services::handle::Serve;
use crate::services::service::Service;
use async_std::sync::Sender;
use std::fmt::Debug;
use std::marker::PhantomData;

/// Trait representing an envelope for a service
#[async_trait::async_trait]
pub(crate) trait ServiceEnvelope: Send + Debug {
    type Service: Service;

    async fn dispatch(&mut self, service: &Self::Service);
}

/// For send without response we can use the normal Letter struct
impl<S: Service + Notify<M>, M: Debug> Letter<S, M> {
    pub fn new_for_service(message: M) -> Self
    where
        S: Notify<M>,
        Self: ServiceEnvelope,
    {
        Letter {
            message: Some(message),
            phantom: PhantomData,
        }
    }

    pub async fn dispatch_service(&mut self, service: &mut S) {
        if let Some(message) = self.message.take() {
            <S as Notify<M>>::handle(service, message).await;
        }
    }
}

/// For send without response we can use the normal Letter struct
#[async_trait::async_trait]
impl<S: Service + Notify<M>, M: Debug + Send> ServiceEnvelope for Letter<S, M> {
    type Service = S;

    async fn dispatch(&mut self, service: &Self::Service) {
        Letter::<S, M>::dispatch(self, service).await;
    }
}

/// For messages with a response we need to use a different structure than LetterWithResponder
#[derive(Debug)]
pub(crate) struct ServiceLetterWithResponders<S: Service + Serve<M>, M: Debug> {
    message: Option<M>,
    responder: Option<Sender<<S as Serve<M>>::Response>>,
    phantom: PhantomData<S>,
}

/// For messages with a response we need to use a different structure than LetterWithResponder
impl<S: Service + Serve<M>, M: Debug> ServiceLetterWithResponders<S, M> {
    fn new(message: M, responder: Sender<<S as Serve<M>>::Response>) -> Self
    where
        S: Serve<M>,
    {
        ServiceLetterWithResponders {
            message: Some(message),
            phantom: PhantomData,
            responder: Some(responder),
        }
    }

    async fn dispatch(&mut self, service: &S) {
        if let Some(message) = self.message.take() {
            if let Some(responder) = self.responder.take() {
                let result = <S as Serve<M>>::handle(service, message).await;
                responder.send(result).await;
            }
        }
    }
}

/// For messages with a response we need to use a different structure than LetterWithResponder
#[async_trait::async_trait]
impl<S: Service + Serve<M>, M: Debug + Send> ServiceEnvelope for ServiceLetterWithResponders<S, M> {
    type Service = S;

    async fn dispatch(&mut self, service: &Self::Service) {
        ServiceLetterWithResponders::<S, M>::dispatch(self, service).await;
    }
}
