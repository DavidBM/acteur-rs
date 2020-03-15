<h1 align="center">Acteur Actor System</h1>
<div align="center">
 <strong>
   An actor system written in Rust that just works. Simple, robust, fast, documented.
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

Acteur uses async_std under the hood. This actor system work under the following premises:

 - **Simplicity**: The API should be small, simple and intuitive
 - **Speed**: The system should be fast and use all available CPU cores
 - **Documented**: Everything must be documented with exhaustive examples

Regarding the implementation:

 - Actors have an ID
 - Messages are routed to an Actor and an ID
 - Actor life-cycle is automatically handled by the framework
 - Actors are automatically de/allocated depending of their usage
 - Messages for the same Actor & ID are ordered. Everything else is executed in concurrently.

## State of the implementation

- [x] Actor is activated on first message
- [x] Actor can send messages to other actors
- [x] System can send messages to any actor
- [ ] System statistics
- [ ] Automatic deallocation of unused actors
- [x] Stop waits for all actors to consume all messages
- [ ] Subscribe to message
- [ ] Fan-out messages
- [x] Actor self stop
- [ ] Allow more than 150.000 queued messages per actor (waiting for async_std to have unbounded channels: [https://github.com/async-rs/async-std/issues/212]())

## Examples

```rust
use acteur::{Actor, Assistant, Handle, System};
use async_trait::async_trait;

#[derive(Debug)]
struct Employee {
    salary: u32,
}

#[async_trait]
impl Actor for Employee {
    type Id = u32;

    async fn activate(_: Self::Id) -> Self {
        Employee {
            salary: 0, //Load from DB or set a default,
        }
    }
}

#[derive(Debug)]
struct SalaryChanged(u32);

#[async_trait]
impl Handle<SalaryChanged> for Employee {
    async fn handle(&mut self, message: SalaryChanged, _: Assistant<Employee>) 
    {
        self.salary = message.0;
    }
}

fn main() {
    let sys = System::new();

    sys.send::<Employee, SalaryChanged>(42, SalaryChanged(55000));

    sys.wait_until_stopped();
}

```

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
