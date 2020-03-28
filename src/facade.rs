use crate::actor_proxy::ActorReport;
use crate::system_director::SystemDirector;
use crate::{Actor, Receive};
use async_std::task;
use lazy_static::lazy_static;
use std::any::TypeId;
use std::fmt::Debug;

// We do this in order to keep all the actors in the same system. If not, two calls
// to "new" can create duplicated actors.
lazy_static! {
    static ref ADDRESS_BOOK: SystemDirector = SystemDirector::new();
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
            system_director: ADDRESS_BOOK.clone(),
        }
    }

    /// Sends a message to an actor with an ID.
    /// If the actor is not loaded in Ram, this method will load them first
    /// by calling their "activate" method.
    pub async fn send<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        self.system_director.send::<A, M>(actor_id, message).await;
    }

    /// Same as `send` method, but sync version.
    pub fn send_sync<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        task::block_on(async move { self.send::<A, M>(actor_id, message).await })
    }

    /// Send an stop message to all actors in the system.
    /// Actors will process all the enqued messages before stop
    pub fn stop(&self) {
        let system = self.system_director.clone();
        task::spawn(async move {
            system.stop().await;
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
