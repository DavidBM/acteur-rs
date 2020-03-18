use acteur::{Acteur, Actor, Assistant, Handle};
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
    async fn handle(&mut self, message: SalaryChanged, _: &Assistant<Employee>) {
        self.salary = message.0;
    }
}

fn main() {
    let sys = Acteur::new();

    sys.send_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));

    sys.wait_until_stopped();
}
