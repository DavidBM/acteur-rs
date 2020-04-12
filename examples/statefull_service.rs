use acteur::{Acteur, Notify, Service, ServiceConfiguration, ServiceAssistant};
use async_std::sync::Mutex;

#[derive(Debug)]
struct EmployeeExpensesCalculator {
    // Services have concurrency, therefore, your state needs to be under a Mutex.
    // Highly recommended using an async Mutex instead of Rust Mutex
    employee_expenses: Mutex<f32>,
}

#[async_trait::async_trait]
impl Service for EmployeeExpensesCalculator {
    async fn initialize(_: &ServiceAssistant<Self>) -> (Self, ServiceConfiguration) {
        println!("Initializing EmployeeExpensesCalculator");

        let service = EmployeeExpensesCalculator {
            employee_expenses: Mutex::new(0.0),
        };

        let service_conf = ServiceConfiguration::default();

        (service, service_conf)
    }
}

#[derive(Debug)]
struct EmployeeHired(f32);

#[async_trait::async_trait]
impl Notify<EmployeeHired> for EmployeeExpensesCalculator {
    async fn handle(&self, message: EmployeeHired, _: &ServiceAssistant<Self>) {
        println!("Adding {} salary to the employee expenses", message.0);
        *self.employee_expenses.lock().await += message.0;
    }
}
fn main() {
    let sys = Acteur::new();
    sys.send_to_service_sync::<EmployeeExpensesCalculator, _>(EmployeeHired(55000.0));
    sys.stop();
    sys.wait_until_stopped();
}
