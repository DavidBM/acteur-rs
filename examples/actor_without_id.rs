// Setting the Actor::Id as "()" we make impossible to have serveral instances of this actor.
//
// This will result on a normal actor with only one instance. There are several things to
// consider in this case:
//
// The actor won't process messages concurrently. As a normal actor instance, it will
// process the messages one by one. I have planned to implement something like "Services"
// that will spawn several instances of the "service" actor and will round robin the
// messages between them.
//
// Until then, this is how to do a id-less Actor.

use acteur::{Acteur, Actor, Assistant, Receive};
use async_trait::async_trait;

#[derive(Debug)]
struct Employee {
    salary: u32,
}

#[async_trait]
impl Actor for Employee {
    // We use the type () as id-less id. Therefore, only one actor will exist
    type Id = ();

    async fn activate(_: Self::Id, _: &Assistant<Self>) -> Self {
        Employee {
            salary: 0, //Load from DB or set a default,
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

    // As drawback, is that we need to write () each time.
    // That will be solved when services are implemented.
    sys.send_to_actor_sync::<Employee, SalaryChanged>((), SalaryChanged(55000));

    sys.wait_until_stopped();
}
