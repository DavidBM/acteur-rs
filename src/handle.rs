use crate::actor_proxy::Secretary;
use crate::Actor;
use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait Handle<T: Debug>: Actor {
    async fn handle(&mut self, message: T, secretary: Secretary);
}
