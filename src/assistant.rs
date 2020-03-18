use crate::actors_manager::ActorManagerProxyCommand;
use crate::address_book::AddressBook;
use crate::envelope::ManagerLetter;
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
/// #     async fn handle(&mut self, message: SayByeForever, assistant: &Assistant<Manager>) {}
/// # }
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[derive(Debug)]
/// struct SayByeForever(String);
///
/// #[async_trait]
/// impl Handle<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, assistant: &Assistant<Employee>) {
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
/// #     sys.send_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
/// #
/// #     sys.wait_until_stopped();
/// # }
///
/// ```
///
pub struct Assistant<A: Actor> {
    address_book: AddressBook,
    actor_id: A::Id,
}

impl<A: Actor> Assistant<A> {
    pub(crate) fn new(address_book: AddressBook, actor_id: A::Id) -> Assistant<A> {
        Assistant {
            address_book,
            actor_id,
        }
    }

    /// Sends a message to the Actor with the specified Id.
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn send<A2: Actor + Handle<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A2::Id,
        message: M,
    ) {
        if let Some(sender) = self.address_book.get::<A2>() {
            sender
                .send(ActorManagerProxyCommand::Dispatch(Box::new(
                    ManagerLetter::<A2, M>::new(actor_id, message),
                )))
                .await;
        }
    }

    /// Enqueues a end command in the Actor messages queue. The actor will consume all mesages before ending.
    /// Keep in mind that event is an actor is stopped, a new message in the future can wake up the actor.
    pub async fn stop(&self) {
        if let Some(sender) = self.address_book.get::<A>() {
            sender
                .send(ActorManagerProxyCommand::EndActor(self.actor_id.clone()))
                .await;
        }
    }

    /// Send an stop message to all actors in the system.
    /// Actors will process all the enqued messages before stop
    pub async fn stop_system(&self) {
        self.address_book.stop_all();
    }
}

impl<A: Actor> Clone for Assistant<A> {
    fn clone(&self) -> Self {
        Assistant {
            address_book: self.address_book.clone(),
            actor_id: self.actor_id.clone(),
        }
    }
}

impl<A: Actor> Debug for Assistant<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActorSecretary for {}", std::any::type_name::<A>())
    }
}
