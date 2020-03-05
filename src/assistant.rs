use crate::actors_manager::ActorManagerProxyCommand;
use crate::envelope::ManagerLetter;
use crate::system::AddressBook;
use crate::{Actor, Handle};
use std::fmt::Debug;

/// This object is provided to the [Handle](./trait.Handle.html) method for each message that an Actor receives
/// The Actor's assistant allows to send messages and to execute some task over the system.
///
/// ```rust,no_run
/// # use acteur::{Actor, Handle, Assistant, System};
/// # use async_trait::async_trait;
/// #
/// # #[derive(Debug)]
/// # struct Employee {
/// #     salary: u32,
/// #     manager_id: u32,
/// # }
/// #
/// # #[async_trait]
/// # impl Actor for Employee {
/// #     type Id = u32;
/// #
/// #     async fn activate(_: Self::Id) -> Self {
/// #         Employee {
/// #             salary: 0, // Load from DB or set a default,
/// #             manager_id: 0 ,
/// #         }
/// #     }
/// # }
/// #
/// # #[derive(Debug)]
/// # struct Manager;
/// #
/// # #[async_trait]
/// # impl Actor for Manager {
/// #     type Id = u32;
/// #
/// #     async fn activate(_: Self::Id) -> Self {
/// #         Manager
/// #     }
/// # }
/// # #[async_trait]
/// # impl Handle<SayByeForever> for Manager {
/// #     async fn handle(&mut self, message: SayByeForever, assistant: Assistant) {}
/// # }
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[derive(Debug)]
/// struct SayByeForever(String);
///
/// #[async_trait]
/// impl Handle<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, assistant: Assistant) {
///         if self.salary > message.0 {
///             assistant.send::<Manager, SayByeForever>(self.manager_id, SayByeForever("Betrayer!".to_string()));
///         }
///         
///         self.salary = message.0;
///     }
/// }
///
/// # fn main() {
/// #     let sys = System::new();
/// #
/// #     sys.send::<Employee, SalaryChanged>(42, SalaryChanged(55000));
/// #
/// #     sys.wait_until_stopped();
/// # }
///
/// ```
///
pub struct Assistant {
    address_book: AddressBook,
}

impl Assistant {
    pub(crate) fn new(address_book: AddressBook) -> Assistant {
        Assistant { address_book }
    }

    /// Sends a message to the Actor with the specified Id.
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn send<A: Actor + Handle<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        if let Some(sender) = self.address_book.get::<A>() {
            sender
                .send(ActorManagerProxyCommand::Dispatch(Box::new(
                    ManagerLetter::<A, M>::new(actor_id, message),
                )))
                .await;
        }
    }

    /// Send an stop message to all actors in the system.
    /// Actors will process all the enqued messages before stop
    pub async fn stop_system(&self) {
        self.address_book.stop_all();
    }
}

impl Clone for Assistant {
    fn clone(&self) -> Self {
        Assistant {
            address_book: self.address_book.clone(),
        }
    }
}

impl Debug for Assistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActorSecretary ()")
    }
}
