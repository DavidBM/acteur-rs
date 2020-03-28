use crate::Actor;
use crate::Assistant;
use async_trait::async_trait;
use std::fmt::Debug;

/// This Trait allow Actors to receive messages. If you want to respond to messages, use the [Respond trait](./trait.HsndleAndRespond.html).
///
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
/// use acteur::{Assistant, Receive, Acteur};
///
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[async_trait]
/// impl Receive<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, _: &Assistant<Employee>) {
///         self.salary = message.0;
///     }
/// }
///
/// fn main() {
///     let sys = Acteur::new();
///
///     sys.send_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
///
///     sys.wait_until_stopped();
/// }
///
/// ```
///
/// You can use the [Assistant](./struct.Assistant.html) in order to interact with other actors and the system.
///
///
#[async_trait]
pub trait Receive<M: Debug>
where
    Self: Sized + Actor,
{
    /// This method is called each time a message is received. You can use the [Assistant](./struct.Assistant.html) to send messages
    async fn handle(&mut self, message: M, assistant: &Assistant<Self>);
}

/*
/// This Trait allow Actors to receive messages and, additionally, respond to them. This trait is like the [Handle trait](./trait.Handle.html) but additionally allos you to respond to messages
///
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
/// use acteur::{Assistant, Handle, Acteur};
///
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[async_trait]
/// impl Respond<SalaryChanged> for Employee {
/// 	type Response = String;
///     async fn handle(&mut self, message: SalaryChanged, _: &Assistant<Employee>) -> String {
///         self.salary = message.0;
/// 		String::from("Thanks!")
///     }
/// }
///
/// fn main() {
///     let sys = Acteur::new();
///
///     sys.send_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
///
///     sys.wait_until_stopped();
/// }
///
/// ```
///
/// You can use the [Assistant](./struct.Assistant.html) in order to interact with other actors and the system.
///
///
#[async_trait]
pub trait Respond<M: Debug>
where
    Self: Sized + Actor,
{
	type Response;
    /// This method is called each time a message is received. You can use the [Assistant](./struct.Assistant.html) to send messages
    async fn handle(&mut self, message: M, assistant: &Assistant<Self>) -> Self::Response;
}*/
