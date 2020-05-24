//! # Acteur Actor System
//!
//! An safe & opinionated actor system written in Rust that just works. Simple, robust, fast, documented.
//!
//! ## Main features of Acteur
//!
//! Acteur uses [`async_std`](https://github.com/async-rs/async-std) under the hood. You can find the all the information in the [documentation](https://docs.rs/acteur).
//! 
//! ## Status update
//! 
//! I taking some time to think better where to fit this framework. In it's current form it is built 
//! to write business logic, but at the same time it remains kind of a low level framework. I will 
//! research and see what are the next steps.
//! 
//! This actor system is a bit different than other frameworks. It work under the following premises:
//!  - **High-level**: The framework is oriented to map business logic rather than concurrent tasks.
//!  - **Simple**: The API should be small, simple and intuitive. No surprises.
//!  - **Fast**: The system should be fast and use all available CPU cores.
//!  - **Documented**: Everything must be documented with exhaustive examples.
//!
//! ### Regarding the implementation:
//!
//!  - Acteur is **asynchronous** and uses `async_std` under the hood. (Even for mutexes)
//!  - Actors have an *ID* which type is defined by the developer.
//!  - Messages are routed to an *Actor* and an *ID* .
//!  - Actor life-cycle is *automatically* managed by the framework.
//!  - Messages for the same Actor & ID are *sequential*. Everything else is executed **concurrently**.
//!  - Services are provided for other concurrency forms.
//!  - Services **don't** have ID and are concurrent.
//!  - Services can **subscribe** to messages and everyone can **publish** messages.
//!  - Acteur is **global**, only one instance can exist.
//!  
//! ### Note about calling Acteur and Actor system
//! 
//! Acteur takes inspiration in Actors but it takes a different path. Actor based concurrency 
//! model is a concurrency modeling tool, not a business logic one, therefore, you cannot expect 
//! to solve logic organization problems with Actors. Still, Actors is a very nice abstraction 
//! that can, tangentially, solve certain nuances of logic organization. Even more when the code scales.
//! 
//! In the way Acteur is implemented it searches to help you splitting your instances and keep 
//! request ordered and concurrent as much as possible between instances.
//! 
//! Said that, Acteur is provably **not** the tool you want if:
//! 
//!  - You want to have a full ACID compliant system
//!  - You want to fully follow the Actor model
//!  - You need to scale to A LOT of traffic. In which case you will need more than one server. (I'm planning to implement some multi-server clustering, but for now, only one server).
//! 
//! But it may help you if you want:
//! 
//!  - To have a database but not incur in the cost of READ, APPLY, SAVE, and instead you want to keep object instances in RAM.
//!  - You don't want to deal with optimistic concurrency and you want the messages to process one by one for each ID, but concurrently between IDs.
//!  - You want to make an backend for an online videogame with many entities interacting at the same time but don't want to go all the way with ECS.
//!
//! ### State of the implementation
//!
//! The overall feature set is complete. Acteur will continue improving
//! and adding improvements. As for now, this framework is actively supported/developed.
//!
//! My main focus of work now is in the ergonomics.
//!
//! - ☑️ Actor / services is activated on first message
//! - ☑️ Actor can send messages to other actors / services
//! - ☑️ System can send messages to any actor / service
//! - ☑️ Actors / Services can optimally, respond to messages
//! - ☑️ Services (statefull or stateless, like actors, without ID and concurrent)
//! - ☑️ Automatic deallocation of unused actors (after 5 minutes without messages)
//! - ☑️ Services can subscribe to messages
//! - □ Actor deallocation configuration (based in RAM, Actor count or timeout)
//!
//! ## Acteur structure
//!
//! In order to use Acteur you just need to implement the correct trait and Acteur will
//! automatically use your implementation when a message is routed to your Actor/Service.
//!
//! The main traits are:
//!
//! - [Actor](./trait.Actor.html): Represents an actor
//! - [Service](./trait.Service.html): Represents a service
//!
//! Just implement them and your Actor/Service is ready to use.
//!
//! For Actors you have two traits in order to handle messages:
//!
//! - [Receive](./trait.Receive.html): Receives a message without responding to it. The most efficient way to handle messages.
//! - [Respond](./trait.Respond.html): Receives a message and allows to respond to it. Forces to sender to await until the actor respond.
//!
//! For Services you have other two traits.
//!
//! - [Listen](./trait.Listen.html): Receives a message without responding to it. The most efficient way to handle messages.
//! - [Serve](./trait.Serve.html): Receives a message and allows to respond to it. Forces to sender to await until the actor respond.
//!
//! #### Why are you using 4 different trait instead of 1 or 2?
//! 
//! I tried to merge Traits but I didn't find how to do it because:
//! 
//! A) The handle method contains the ActorAssistant and ServiceAssistant types in the signatures, witch have different types.
//! B) I don't like to create a response channel for EVERY message when many messages don't need a response.
//! 
//! Both blockes make 4 combinations. Receive/Respond for Actors and Listen/Serve for Services.
//! 
//! I'm still trying to improve the naming and ergonimocs. I think the concept will remain, but the ergonomics may change a bit. 
//! 
//! ## Actors & Services
//!
//! Acteur provides 2 ways of concurrency. Actors and Services.
//!
//! ### Actors
//!
//! Actors have an ID and will consume messages directed to the same Actor's ID sequentially.
//! That means that if you send 2 messages to the Actor User-32, they will be handled sequentially.
//! On the other side, if you send a message to the Actor User-32 and other to the User-52 the
//! messages will be handled concurrently.
//!
//! That means, Actors instances keep messages order for the same ID, but not between different IDs.
//!
//! ### Services
//!
//! Services, on the other side, have no ID and they are concurrent. That means that you choose
//! how many instances of the Service there will be (Acteur provides a default). Services can
//! or can't have an State, but if they have, they require to be Sync (aka Mutex<state>).
//!
//! In short. Services are like normal web services. You can have many instances and there is
//! no synchronization of any type when consuming messages. Think of them as the primitive you
//! use when you want to create something that doesn't fit the Actors model in this framework.
//!
//! ### Use cases
//!
//! Choose Actor for Entities (Users, Invoices, Players, anything which their instances are identified).
//!
//! Choose Services for Business Logic, Infrastructure, Adapters, etc (Storage, DB access, HTTP services,
//! calculations of some sort that doesn't belong to any Actor, etc) and for subscribing to messages (Pub/Sub)
//!
//! ## Subscription or Pub/Sub
//!
//! Sometime we don't want to know who should receive the message but to subscribe to a type and wait.
//! Acteur models the Pub/Sub patter with Services. Actors in Acteur can't perform subscriptions as
//! that would require the framework to know all possible IDs of all possible Actor instances in
//! order to direct the message to the correct one (or all) and it doesn't play well with the deallocation
//! of unused actors.
//!
//! If you want to send messages to some Actors from a Subscription, you can create a Service that
//! subscribes to a message and then figures out to what Actor IDs to send the message. For example,
//! doing a query in the DB/Service in order to get the set of IDs that need to receive some message.
//!
//! Unlike sending/calling to services/actors, publishing doesn't know who needs to receive the
//! message in compilation time. That is the reason behind requiring the Services to subscribe in
//! runtime to any message they want to receive. In order to ensure that services perform the
//! subscriptions, it is a good idea to run `acteur.preload_service<Service>();` for each service
//! that should perform any subscription at the beginning of your Application start.
//!
//! ## Simple Example
//!
//! ```rust,no_run
//! use acteur::{Actor, Receive, ActorAssistant, Acteur};
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
//!     async fn activate(_: Self::Id, _: &ActorAssistant<Self>) -> Self {
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
//!     async fn handle(&mut self, message: SalaryChanged, _: &ActorAssistant<Employee>) {
//!         self.salary = message.0;
//!     }
//! }
//!
//! fn main() {
//!     let sys = Acteur::new();
//!
//!     sys.send_to_actor_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
//!
//!     sys.wait_until_stopped();
//! }
//!
//! ```
//!
//! ## Why another Actors framework?
//!
//! Initially there were three things that bothered me.
//!
//! 1. Actor systems seem to be low level. If you want "Instances" you need to implement
//! them on top of the framework.
//! 2. In order to use Actix you need to learn how it works. You need to manage the concurrency,
//! the addresses, etc
//! 3. Unsafe. I don't want unsafe. I wouldn't trust myself to do something like this in C++,
//! therefore, I don't want to have unsafe code. Rust opens the door to do these kind of projects
//! to people with less than 10 years of experience in C/C++ in a safer way.
//!
//! After async_std 1.0 announcement and speaking with some friends I started to envision how I would
//! like an actor framework be. Not that Actix and others are wrong, but they are too low level in my
//! opinion. I wanted something that just runs without leaking so many underlying concepts. At the same
//! time I don't think that competing for the last nanosecond is healthy. Even less if the framework is
//! already super fast.
//!
//! ## Common patterns
//!
//! This section will be updated with common patters you can use in your applications. If
//! you have one you want to add or just a question of how to so something, let me know with a GitHub Issue.
//!
//! ### Web server
//!
//! Given that all actors are managed by the framework, it is really easy to have, for
//! example, Rocket or Tide getting new HTTP calls and just calling `acteur.call_service` or
//! `acteur.call_actor` and wait for the response. You can use the sync version of the call
//! if you are working with synchronous code. Keep in mind that you can clone Acteur and send
//! it to as many threads/struct you need.
//!
//! ```rust,no_run
//!
//! use acteur::Acteur;
//!
//! let acteur = Acteur::new();
//!
//! // You can clone and send it to another thread/struct
//! let acteur2 = acteur.clone();
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

pub use facade::Acteur;

pub use actors::actor::Actor;
pub use actors::assistant::ActorAssistant;
pub use actors::handle::{Receive, Respond};

pub use services::service::{Service, ServiceConcurrency, ServiceConfiguration};
pub use services::system_facade::ServiceAssistant;
pub use services::handle::{Listen, Serve};
