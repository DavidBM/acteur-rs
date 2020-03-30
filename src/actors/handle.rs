use crate::Actor;
use crate::Assistant;
use async_trait::async_trait;
use std::fmt::Debug;

/// This Trait allow Actors to receive messages. 
///
/// If you want to respond to messages, use the [Respond trait](./trait.Respond.html).
///
/// This trait is compatible with [Respond trait](./trait.Receive.html) as you can implement, for the same message, 
/// both traits. This trait will be executed when using the "send" or "send_sync" method from System or the "send" 
/// method from Assistant.
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

/// This Trait allow Actors to receive messages and, additionally, respond to them.
/// 
/// This trait is like the [Receive trait](./trait.Receive.html) but additionally allows responding to messages
/// 
/// This trait is compatible with [Receive trait](./trait.Receive.html) as you can implement, for the same message, 
/// both traits. This trait will be executed when using the "call" or "call_sync" method from System or the "call" 
/// method from Assistant.
/// 
/// ## Note about concurrency and performance
/// 
/// If there is no reason to respond to a message, prefer to use the Receice trait, as this may delay you Actors or 
/// even deadlock them. For example:
/// 
///  - Actor A-51 calls Actor B-32 
///  - Actor B-32 calls Actor A-51 
/// 
/// As Actor A-51 is blocked waiting for B-32, this will keep waiting forever.
/// 
/// Additionally, all the time waiting for a response, is time that the Actor instance won't be processing messages.
/// Keep this in mind as if you call to a very busy instance, that may slow down other instance, making the first wait
/// until the busy ones processes all the messages before yours.
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
/// use acteur::{Assistant, Respond, Acteur};
///
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[async_trait]
/// impl Respond<SalaryChanged> for Employee {
///     type Response = String;
///     async fn handle(&mut self, message: SalaryChanged, _: &Assistant<Employee>) -> String {
///         self.salary = message.0;
///         String::from("Thanks!")
///     }
/// }
///
/// fn main() {
///     let sys = Acteur::new();
///
///     let response = sys.call_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
///
///     println!("Response is: {:?}", response); // Response is: Thanks!
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
pub trait Respond<M: Debug>: Sized + Actor {
    type Response: Send;
    /// This method is called each time a message is received. You can use the [Assistant](./struct.Assistant.html) to send messages
    async fn handle(&mut self, message: M, assistant: &Assistant<Self>) -> Self::Response;
}
