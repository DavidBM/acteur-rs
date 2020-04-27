<h1 align="center">Acteur Actor System</h1>
<div align="center">
 <strong>
   A safe actor system written in Rust that just works. Simple, robust, fast, documented.
 </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/acteur">
    <img src="https://img.shields.io/crates/v/acteur.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/acteur">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>


## Main Features

Acteur uses [`async_std`](https://github.com/async-rs/async-std) under the hood. You can find the all the information in the [documentation](https://docs.rs/acteur).

This actor system work under the following premises:
 - **Simple**: The API should be small, simple and intuitive. No surprises.
 - **Fast**: The system should be fast and use all available CPU cores.
 - **Documented**: Everything must be documented with exhaustive examples.

### Regarding the implementation:

 - Acteur is **asynchronous** and uses `async_std` under the hood. (Even for mutexes)
 - Actors have an *ID* which type is defined by the developer.
 - Messages are routed to an *Actor* and an *ID.
 - Actor life-cycle is *automatically* managed by the framework.
 - Messages for the same Actor & ID are *sequential*. Everything else is executed **concurrently**.
 - Services are provided for other concurrency forms.
 - Services **don't** have ID and are concurrent.
 - Services can **subscribe** to messages and everyone can **publish** messages.
 - Acteur is **global**, only one instance can exist. 

### State of the implementation

The overall feature set is complete. Acteur will continue improving 
and adding improvements. As for now, this framework is actively supported/developed.

The current focus is on ergonomics and use cases. Hopefully we can have an stable API soon.

- ☑️ Actor / services is activated on first message
- ☑️ Actor can send messages to other actors / services
- ☑️ System can send messages to any actor / service
- ☑️ Actors / Services can optimally, respond to messages
- ☑️ Services (statefull or stateless, like actors, without ID and concurrent)
- ☑️ Automatic deallocation of unused actors (after 5 minutes without messages)
- ☑️ Services can subscribe to messages
- ☑️ Different concurrency modes for services (fixed, core cound, unlimited, etc)

## Examples

Simple example

```rust
use acteur::{Actor, Receive, Assistant, Acteur};
use async_trait::async_trait;

#[derive(Debug)]
struct Employee {
    salary: u32
}

#[async_trait]
impl Actor for Employee {
    type Id = u32;
    async fn activate(_: Self::Id, _: &Assistant<Self>) -> Self {
        Employee {
            salary: 0 // Load from DB or set a default,
        }
    }
}

#[derive(Debug)]
struct SalaryChanged(u32);

#[async_trait]
impl Receive<SalaryChanged> for Employee {
    async fn handle(&mut self, message: SalaryChanged, _: &Assistant<Employee>) {
        self.salary = message.0;
    }
}

fn main() {
  let sys = Acteur::new();

  sys.send_to_actor_sync::<Employee, _>(42, SalaryChanged(55000));

  sys.wait_until_stopped();
}

```

You can find more examples in [the examples folder](./examples).

## Safe Rust

No unsafe code was directly used in this crate. You can check in lib.rs the `#![deny(unsafe_code)]` line.

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
