use async_trait::async_trait;
use std::fmt::Debug;
use std::hash::Hash;

/// The mail Trait from this crate.
///
/// This this Trait you define enable your structs to be used as actors.
/// You will need to use [`Handle`](./trait.Handle.html) Trait in order to accept messages.
///
/// Actors are required to contain an Id, this Id type is defined by you.
/// The only constrains for such Id are `Eq + Hash + Send + Sync + Clone + Debug`
///
/// ```rust,no_run
/// use acteur::{Actor, Assistant};
/// use async_trait::async_trait;
///
/// // You can use any normal struct as an actor. It will contain the actor state..
/// #[derive(Debug)]
/// struct Employee {
/// 	id: u32,
///     salary: u32,
/// }
///
/// #[async_trait]
/// impl Actor for Employee {
///     type Id = u32;
///
/// 	// You can use or not the actor Id, still, it will be kept by the framework.
/// 	// This method allows you to acquire any resource you need and save it.
///     async fn activate(id: Self::Id) -> Self {
/// 		println!("Employee {:?} activated!", id);
///         Employee {
/// 			id,
///             salary: 0 //Load from DB, set a default, etc
///         }
///     }
///
/// 	// You can use or not the actor Id, still, it will be kept by the framework.
/// 	// This method allows you to acquire any resource you need and save it.
///     async fn deactivate(&mut self) {
/// 		println!("Employee {:?} deactivated!", self.id);
///     }
/// }
/// ```
///
#[async_trait]
pub trait Actor: Debug + Send + Sync + 'static {
    /// The Id type for the actor.
    /// This Id will be used to identify the actor internally.
    type Id: Eq + Hash + Send + Sync + Clone + Debug;

    /// This method will be called automatically when the actor is activated.
    /// Normally, actors are activated when the first message is received.
    async fn activate(id: Self::Id) -> Self;

    /// This method will be called when the framework decided to unload the actor.
    /// The concrete algorithms to decide that can change in the future.
    /// As for now, this method is never called and actors are never unloaded.
    async fn deactivate(&mut self) {}
}
