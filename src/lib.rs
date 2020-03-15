//! # Acteur Actor System
//!
//! An actor system written in Rust that just works. Simple, robust, fast, documented.
//!
//! ## Overall features of Acteur
//!
//! ## Example
//!
//! ```rust,no_run
//! use acteur::{Actor, Handle, Assistant, System};
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
//! impl Handle<SalaryChanged> for Employee {
//!     async fn handle(&mut self, message: SalaryChanged, _: Assistant) {
//!         self.salary = message.0;
//!     }
//! }
//!
//! # fn main() {
//! let sys = System::new();
//!
//! sys.send::<Employee, SalaryChanged>(42, SalaryChanged(55000));
//!
//! sys.wait_until_stopped();
//! # }
//!
//! ```
//!

mod actor;
mod actor_proxy;
mod actors_manager;
mod assistant;
mod envelope;
mod handle;
mod address_book;
mod system;

pub use actor::Actor;
pub use assistant::Assistant;
pub use handle::Handle;
pub use system::System;
