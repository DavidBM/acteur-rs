use crate::address_book::{AddressBook, WaitSystemStop};
use crate::actors_manager::{ActorManagerProxyCommand};
use crate::envelope::ManagerLetter;
use crate::{Actor, Handle};
use async_std::{
    task::spawn,
};
use std::fmt::Debug;

/// The system is external inteface to the actor runtime.
/// It allow to send messages, to stop it, configure it, etc.
/// Once you contructed with the method "new" you can start sending messages.
/// The system will automatically start any required actor automatically and unload them when required.
pub struct System {
    address_book: AddressBook,
}

impl Default for System {
    fn default() -> Self {
        System::new()
    }
}

impl System {
    /// Initializes the system. After this, you can send messages using the send method.
    pub fn new() -> System {
        let address_book = AddressBook::new();
        System { address_book }
    }

    /// Sends a message to an actor with an ID.
    /// If the actor is not loaded in Ram, this method will load them first
    /// by calling their "activate" method.
    pub fn send<A: Actor + Handle<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        if let Some(sender) = self.address_book.get::<A>() {
            spawn(async move {
                sender
                    .send(ActorManagerProxyCommand::Dispatch(Box::new(
                        ManagerLetter::new(actor_id, message),
                    )))
                    .await;
            });
        }
    }

    /// Send an stop message to all actors in the system.
    /// Actors will process all the enqued messages before stop
    pub fn stop(&self) {
        self.address_book.stop_all();
    }

    /// Waits until all actors are stopped.
    /// If you call "system.stop()" this method will wait untill all actor
    /// have consumed all messages before returning.
    pub fn wait_until_stopped(&self) {
        async_std::task::block_on(async {
            WaitSystemStop::new(self.address_book.clone()).await;
        });
    }
}

impl Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActeurSystem ()")
    }
}

impl Clone for System {
    fn clone(&self) -> Self {
        System {
            address_book: self.address_book.clone(),
        }
    }
}
