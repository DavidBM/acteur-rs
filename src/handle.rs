use crate::Actor;
use crate::Assistant;
use async_trait::async_trait;
use std::fmt::Debug;

/// This Trait allow Actors to receive messages.
///
/// Following the example in the [Actor documentation](./trait.Actor.html)...
/// ```rust,no_run
/// use async_trait::async_trait;
/// # use acteur::{Actor};
/// # #[derive(Debug)]
/// # struct Employee {
/// #     id: u32,
/// #     salary: u32,
/// # }
/// #
/// # #[async_trait]
/// # impl Actor for Employee {
/// #     type Id = u32;
/// #
/// #     async fn activate(id: Self::Id) -> Self {
/// #         println!("Employee {:?} activated!", id);
/// #         Employee {
/// #             id,
/// #             salary: 0 //Load from DB, set a default, etc
/// #         }
/// #     }
/// #
/// #     async fn deactivate(&mut self) {
/// #         println!("Employee {:?} deactivated!", self.id);
/// #     }
/// # }
/// use acteur::{Assistant, Handle, System};
///
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[async_trait]
/// impl Handle<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, _: Assistant) {
///         self.salary = message.0;
///     }
/// }
///
/// fn main() {
///     let sys = System::new();
///
///     sys.send::<Employee, SalaryChanged>(42, SalaryChanged(55000));
///
///     sys.wait_until_stopped();
/// }
///
/// ```
///
/// This actor will send messages counting until one million and then stop the system.
///
/// You can use the [Assistant](./struct.Assistant.html) in order to interact with other actors and the system.
///
/// Note: You usually don't call stop_system in there, as it will stop the whole actor framework.
///
#[async_trait]
pub trait Handle<M: Debug>
where
    Self: Sized + Actor,
{
    /// This method is called each time a message is received. You can use the [Assistant](./struct.Assistant.html) to send messages
    async fn handle(&mut self, message: M, assistant: Assistant<Self>);
}
