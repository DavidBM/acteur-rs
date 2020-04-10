//! # Acteur Actor System
//!
//! An actor system written in Rust that just works. Simple, robust, fast, documented.
//!
//! ## Overall features of Acteur
//!
//! Acteur uses async_std under the hood. This actor system work under the following premises:
//!
//!  - **Simplicity**: The API should be small, simple and intuitive
//!  - **Speed**: The system should be fast and use all available CPU cores
//!  - **Documented**: Everything must be documented with exhaustive examples
//!
//! Regarding the implementation:
//!
//!  - Actors have an ID which type is defined by the user for each Actor type
//!  - Messages are routed to an Actor and an ID
//!  - Actor life-cycle is automatically handled by the framework
//!  - Actors are automatically de/allocated depending of their usage
//!  - Messages for the same Actor & ID are ordered. Everything else is executed concurrently.
//!
//! ### State of the implementation
//!
//! - ☑️ Actor is activated on first message
//! - ☑️ Actor can send messages to other actors
//! - ☑️ System can send messages to any actor
//! - ☑️ Actor self stop
//! - ☑️ Stop waits for all actors to consume all messages
//! - ☑️ System statistics
//! - ☑️ RPC like messages between actors
//! - ☑️ Services (statefull or stateless, like actors, without ID and processing messages concurrently)
//! - ☑️ Automatic deallocation of unused actors
//! - □ Actor deallocation configuration (based in RAM, Actor count or timeout)
//! - □ Subscribe to message
//! - □ Fan-out messages
//! - □ Allow more than 150.000 queued messages per actor (waiting for async_std to have unbounded channels: [https://github.com/async-rs/async-std/issues/212]())
//!
//! ## Actors & Services
//!
//! Acteur provides 2 ways of concurrency. Actors and Services. 
//!
//! ### Actors
//!
//! Actors have an ID and will consume messages directed to the same Actor's ID sequentially. That means that if you have
//! if you send 2 messages to the Actor User-32, they will be executed in order. On the other side, if you send a message 
//! to the Actor User-32 and other to the User-52 they will consume the messages concurrently.
//! 
//! That means, Actors instances keep the messages order for the same ID, but not between different IDs. 
//! 
//! ### Services
//! 
//! Services, on the other side, have no ID and they have concurrency. That means that you choose how many instances of 
//! the Service there will be (Acteur provides a default). Services can or can't have an State, but if they have, they 
//! require to be Sync (aka Mutex<state>). 
//! 
//! In short. Services are like services in servers. You can have many instances and there is no sychronization of any
//! type when consuming messages.
//! 
//! ### Use cases
//! 
//! Choose Actor for Entities (Users, Invoices, Players, anything which their instances are identified).
//! 
//! Choose Services for Business Logic, Infrastructure, Adaptors, etc (Storage, DB access, HTTP services, 
//! calculations of some sort that doesn't belong to any Actor, etc)
//! 
//! ## Simple Example
//!
//! ```rust,no_run
//! use acteur::{Actor, Receive, Assistant, Acteur};
//! use async_trait::async_trait;
//!
//! #[derive(Debug)]
//! struct Employee {
//!     salary: u32
//! }
//!
//! #[async_trait]
//! impl Actor for Employee {
//!     type Id = u32;
//!
//!     async fn activate(_: Self::Id) -> Self {
//!         Employee {
//!             salary: 0 // Load from DB or set a default,
//!         }
//!     }
//! }
//!
//! #[derive(Debug)]
//! struct SalaryChanged(u32);
//!
//! #[async_trait]
//! impl Receive<SalaryChanged> for Employee {
//!     async fn handle(&mut self, message: SalaryChanged, _: &Assistant<Employee>) {
//!         self.salary = message.0;
//!     }
//! }
//!
//! # fn main() {
//! let sys = Acteur::new();
//!
//! sys.send_to_actor_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
//!
//! sys.wait_until_stopped();
//! # }
//!
//! ```
//!

#![deny(unsafe_code)]

#[macro_use]
mod utils;
mod actors;
mod facade;
mod services;
mod system_director;

pub use actors::actor::Actor;
pub use actors::assistant::Assistant;
pub use actors::handle::{Receive, Respond};
pub use facade::Acteur;
pub use services::handle::{Notify, Serve};
pub use services::service::{Service, ServiceConcurrency, ServiceConfiguration};
pub use services::system_facade::System;
