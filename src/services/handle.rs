use crate::services::service::Service;
use crate::Actor;
use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait Notify<M: Debug>
where
    Self: Sized + Service,
{
    async fn handle(&self, message: M);
}

#[async_trait]
pub trait Serve<M: Debug>: Sized + Actor {
    type Response: Send;

    async fn handle(&self, message: M) -> Self::Response;
}
