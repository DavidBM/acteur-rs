use crate::Actor;
use crate::ActorAssistant;
use async_trait::async_trait;
use std::fmt::Debug;

/// This Trait allow Actors to receive messages. This is the most efficient way to process messages as it doesn't
/// require to respond the message.
///
/// If you want to respond to messages, use the [Respond trait](./trait.Respond.html).
///
/// This trait is compatible with [Respond trait](./trait.Respond.html) as you can implement, for the same message,
/// both traits. This trait will be executed when using the "send_to_actor" or "send_to_actor_sync" method from System 
/// or the "send_to_actor" method from ActorAssistant.
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
/// #     async fn activate(id: Self::Id, _: &ActorAssistant<Self>) -> Self {
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
/// use acteur::{ActorAssistant, Receive, Acteur};
///
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[async_trait]
/// impl Receive<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, _: &ActorAssistant<Employee>) {
///         self.salary = message.0;
///     }
/// }
///
/// fn main() {
///     let sys = Acteur::new();
///
///     sys.send_to_actor_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
///
///     sys.wait_until_stopped();
/// }
///
/// ```
///
///
#[async_trait]
pub trait Receive<M: Debug>
where
    Self: Sized + Actor,
{
    /// This method is called each time a message is received.
    /// You can use the [ActorAssistant](./struct.ActorAssistant.html) to send messages from actors,
    /// [`System`](./struct.System.html) to send messages from services and
    /// [`Acteur`](./struct.System.html) to send messages from outside the framework
    async fn handle(&mut self, message: M, assistant: &ActorAssistant<Self>);
}

/// This Trait allow Actors to receive messages and, additionally, respond to them.
///
/// This trait is like the [Receive trait](./trait.Receive.html) but additionally allows responding to messages
///
/// This trait is compatible with [Receive trait](./trait.Receive.html) as you can implement, for the same message,
/// both traits. This trait will be executed when using the "call_actor" or "call_actor_sync" method from System or
/// the "call_actor" method from ActorAssistant.
///
/// ## Note about concurrency and performance
///
/// If there is no reason to respond to a message, prefer to use the [Receive trait](./trait.Receive.html) trait, as 
/// this may delay you Actors or even deadlock them. For example:
///
///  1. Actor A-51 calls Actor B-32
///  2. Actor B-32 calls Actor A-51
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
/// #     async fn activate(id: Self::Id, _: &ActorAssistant<Self>) -> Self {
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
/// use acteur::{ActorAssistant, Respond, Acteur};
///
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[async_trait]
/// impl Respond<SalaryChanged> for Employee {
///     type Response = String;
///     async fn handle(&mut self, message: SalaryChanged, _: &ActorAssistant<Employee>) -> String {
///         self.salary = message.0;
///         String::from("Thanks!")
///     }
/// }
///
/// fn main() {
///     let sys = Acteur::new();
///
///     let response = sys.call_actor_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
///
///     println!("Response is: {:?}", response); // Response is: Thanks!
///
///     sys.wait_until_stopped();
/// }
///
/// ```
///
/// You can use the [ActorAssistant](./struct.ActorAssistant.html) in order to interact with other actors and the system.
///
///
#[async_trait]
pub trait Respond<M: Debug>: Sized + Actor {
    type Response: Send;
    /// This method is called each time a message is received.
    /// You can use the [ActorAssistant](./struct.ActorAssistant.html) to send messages from actors,
    /// [`System`](./struct.System.html) to send messages from services and
    /// [`Acteur`](./struct.System.html) to send messages from outside the framework
    async fn handle(&mut self, message: M, assistant: &ActorAssistant<Self>) -> Self::Response;
}
