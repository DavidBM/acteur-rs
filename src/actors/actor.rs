use crate::actors::assistant::ActorAssistant;
use async_trait::async_trait;
use std::fmt::Debug;
use std::hash::Hash;

/// The main Trait from this crate.
///
/// This Trait enable your structs to be used as actors.
/// You will need to use [`Handle`](./trait.Handle.html) or [`Respond`](./trait.Respond.html) Traits in order to accept messages.
///
/// Actors are required to have an Id which type is defined by the developer.
/// The constrains for such Id are `Eq + Hash + Send + Sync + Clone + Debug`
///
/// ```rust,no_run
/// use acteur::{Actor, ActorAssistant};
/// use async_trait::async_trait;
///
/// // You can use any normal struct as an actor. It will contain the actor state. No Arc/Mutex
/// // is required as only one message per instance (different Id) will be handled.
/// #[derive(Debug)]
/// struct Employee {
///     id: u32,
///     salary: u32,
/// }
///
/// #[async_trait]
/// impl Actor for Employee {
///     type Id = u32;
///
///     // You can use or not the actor Id, still, it will be kept by the framework.
///     // This method allows you to acquire any resource you need and save it.
///     async fn activate(id: Self::Id, _: &ActorAssistant<Self>) -> Self {
///         println!("Employee {:?} activated!", id);
///         Employee {
///             id,
///             salary: 0 //Load from DB, set a default, etc
///         }
///     }
///
///     // This method is optional and allows you to delete resources, close sockets, etc.
///     async fn deactivate(&mut self) {
///         println!("Employee {:?} deactivated!", self.id);
///     }
/// }
/// ```
///
#[async_trait]
pub trait Actor: Sized + Debug + Send + Sync + 'static {
    /// The Id type for the actor.
    /// This Id will be used to identify the actor internally.
    type Id: Eq + Hash + Send + Sync + Clone + Debug + Default;

    /// This method will be called automatically when the actor is activated.
    /// Normally, actors are activated when the first message is received.
    async fn activate(id: Self::Id, assistant: &ActorAssistant<Self>) -> Self;

    /// This method will be called when the framework decided to unload the actor.
    /// The concrete algorithms to decide that can change in the future.
    /// As for now, this method is never called and actors are never unloaded.
    async fn deactivate(&mut self) {}
}
