use acteur::{Acteur, Notify, Service, ServiceAssistant, ServiceConfiguration};

#[derive(Debug)]
struct EmployeeTaxesCalculator {
    tax_rate: f32,
}

#[async_trait::async_trait]
impl Service for EmployeeTaxesCalculator {
    async fn initialize(system: &ServiceAssistant<Self>) -> (Self, ServiceConfiguration) {
        let service = EmployeeTaxesCalculator { tax_rate: 0.21 };
        let service_conf = ServiceConfiguration::default();

        println!("Service starting!");

        // Remember to subscribe in order to receiver messages
        system.subscribe::<EmployeeSalaryChange>().await;
        println!("Service subscribed!");

        (service, service_conf)
    }
}

#[derive(Debug, Clone)]
struct EmployeeSalaryChange(f32);

// Subscription uses the trait Notify, as normal message sends
#[async_trait::async_trait]
impl Notify<EmployeeSalaryChange> for EmployeeTaxesCalculator {
    async fn handle(&self, message: EmployeeSalaryChange, _: &ServiceAssistant<Self>) {
        println!("Message received! {:?}", message);

        // Calculate the tax rate and send it to however you want
    }
}

fn main() {
    let mut sys = Acteur::new();

    sys.preload_service_sync::<EmployeeTaxesCalculator>();

    // You just need to send the message, whoever is subscribed will received.
    sys.publish_sync(EmployeeSalaryChange(55000.0));

    sys.stop();

    sys.wait_until_stopped();
}
