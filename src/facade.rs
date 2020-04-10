use crate::actors::proxy::ActorReport;
use crate::services::handle::{Notify, Serve};
use crate::services::service::Service;
use crate::system_director::SystemDirector;
use crate::{Actor, Receive, Respond};
use async_std::task;
use lazy_static::lazy_static;
use std::any::TypeId;
use std::fmt::Debug;

// We do this in order to keep all the actors in the same system. If not, two calls
// to "new" can create duplicated actors.
lazy_static! {
    static ref SYSTEM_DIRECTOR: SystemDirector = SystemDirector::new();
}

/// Acteur is the main inteface to the actor runtime.
/// It allows sending messages, stopping the runtime, set configurations, etc.
/// Once contructed with the method "new" you can start sending messages.
/// The system will automatically start any required actor and unload them when not used.
pub struct Acteur {
    system_director: SystemDirector,
}

impl Default for Acteur {
    fn default() -> Self {
        Acteur::new()
    }
}

impl Acteur {
    /// Initializes the system. After this, you can send messages using the send method.
    pub fn new() -> Acteur {
        Acteur {
            system_director: SYSTEM_DIRECTOR.clone(),
        }
    }

    /// Sends a message to an actor with an ID.
    ///
    /// This method will execute the [Receive::handle](./trait.Receive.html) implemented for
    /// that Message and Actor.
    ///
    /// If the actor is not loaded in Ram, this method will load them first
    /// by calling their "activate" method.
    pub async fn send_to_actor<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        self.system_director
            .send_to_actor::<A, M>(actor_id, message)
            .await;
    }

    /// Same as `send_to_actor` method, but sync version.
    pub fn send_to_actor_sync<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        task::block_on(async move { self.send_to_actor::<A, M>(actor_id, message).await })
    }

    /// As send_to_actor method, it sends a message to an actor with an ID but this one
    /// wait for a response from the actor.
    ///
    /// This method will execute the [Respond::handle](./trait.Respond.html) implemented for
    /// that Message and Actor.
    ///
    /// If the actor is not loaded in Ram, this method will load them first
    /// by calling their "activate" method.
    pub async fn call_actor<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        self.system_director
            .call_actor::<A, M>(actor_id, message)
            .await
    }

    /// Same as `call_actor` method, but sync version.
    pub fn call_actor_sync<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        task::block_on(async move { self.call_actor::<A, M>(actor_id, message).await })
    }

    /// Sends a message to a Service.
    ///
    /// This method will execute the [Notify::handle](./trait.Notify.html) implemented for
    /// that Message and Service.
    ///
    /// If the Service is not loaded in Ram, this method will load them first
    /// by calling their "initialize" method.
    pub async fn send_to_service<S: Service + Notify<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) {
        self.system_director.send_to_service::<S, M>(message).await;
    }

    /// Same as `send_to_service` method, but sync version.
    pub fn send_to_service_sync<S: Service + Notify<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) {
        task::block_on(async move { self.send_to_service::<S, M>(message).await })
    }

    /// As send_to_service method, it sends a message to a Service but this one
    /// wait for a response from the actor.
    ///
    /// This method will execute the [Serve::handle](./trait.Serve.html) implemented for
    /// that Message and Service.
    ///
    /// If the Service is not loaded in Ram, this method will load them first
    /// by calling their "initialize" method.
    pub async fn call_service<S: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<S as Serve<M>>::Response, &str> {
        self.system_director.call_service::<S, M>(message).await
    }

    /// Same as `call_service` method, but sync version.
    pub fn call_service_sync<S: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<S as Serve<M>>::Response, &str> {
        task::block_on(async move { self.call_service::<S, M>(message).await })
    }

    /// Send an stop message to all actors in the system.
    /// Actors will process all the enqued messages before stop
    pub fn stop(&self) {
        let system = self.system_director.clone();
        task::spawn(async move {
            system.stop().await;
        });
    }

    /// Ensures a service is loaded and running.
    /// It ensures that all Service's subscriptions are performed
    pub async fn preload_service<S: Service>(&self) {
        self.system_director.preload_service::<S>().await;
    }

    /// Same as `preload_service` but sync version
    pub async fn preload_service_sync<S: Service>(&self) {
        let system = self.system_director.clone();
        task::block_on(async move {
            system.preload_service::<S>().await;
        });
    }

    /// Waits until all actors are stopped.
    /// If you call "system.stop()" this method will wait untill all actor
    /// have consumed all messages before returning.
    pub fn wait_until_stopped(&self) {
        task::block_on(async { self.system_director.wait_until_stopped().await });
    }

    pub fn get_statistics(&self) -> Vec<(TypeId, Vec<ActorReport>)> {
        self.system_director.get_statistics()
    }
}

impl Debug for Acteur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Acteur ()")
    }
}

impl Clone for Acteur {
    fn clone(&self) -> Self {
        Acteur {
            system_director: self.system_director.clone(),
        }
    }
}
