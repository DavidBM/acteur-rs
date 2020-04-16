use acteur::{Acteur, Actor, Assistant, Receive, Respond};
use async_trait::async_trait;

#[derive(Debug)]
struct Employee {
    id: u32,
    salary: u32,
}

#[async_trait]
impl Actor for Employee {
    type Id = u32;

    async fn activate(id: Self::Id, _: &Assistant<Self>) -> Self {
        println!("Employee {:?} activated!", id);
        Employee {
            id,
            salary: 0, //Load from DB, set a default, etc
        }
    }

    async fn deactivate(&mut self) {
        println!("Employee {:?} deactivated!", self.id);
    }
}

#[derive(Debug)]
struct SalaryChanged(u32);

#[async_trait]
impl Respond<SalaryChanged> for Employee {
    type Response = String;

    async fn handle(&mut self, message: SalaryChanged, assistant: &Assistant<Employee>) -> String {
        // This doesn't make much sense as you can just `self.salary = message.0;` here,
        // but we want to show how you can implement Receive and Response trait for the
        // same message.
        assistant
            .send_to_actor::<Employee, SalaryChanged>(self.id, message)
            .await;

        String::from("Thanks!")
    }
}

// You can have both trait implemented at the same time. This allows you choose when to
// hit the response performance penalty and when now.
#[async_trait]
impl Receive<SalaryChanged> for Employee {
    async fn handle(&mut self, message: SalaryChanged, _: &Assistant<Employee>) {
        self.salary = message.0;
    }
}

fn main() {
    let sys = Acteur::new();

    let response = sys.call_actor_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));

    println!("Response is: {:?}", response); // Response is: Thanks!

    sys.stop();

    sys.wait_until_stopped();
}
