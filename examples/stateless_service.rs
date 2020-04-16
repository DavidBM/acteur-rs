use acteur::{Acteur, Serve, Service, ServiceAssistant, ServiceConfiguration};

#[derive(Debug)]
struct EmployeeTaxesCalculator {
    tax_rate: f32,
}

#[async_trait::async_trait]
impl Service for EmployeeTaxesCalculator {
    async fn initialize(_: &ServiceAssistant<Self>) -> (Self, ServiceConfiguration) {
        let service = EmployeeTaxesCalculator { tax_rate: 0.21 };
        let service_conf = ServiceConfiguration::default();
        (service, service_conf)
    }
}

#[derive(Debug)]
struct EmployeeSalaryChange(f32);

#[async_trait::async_trait]
impl Serve<EmployeeSalaryChange> for EmployeeTaxesCalculator {
    type Response = f32;
    async fn handle(&self, message: EmployeeSalaryChange, _: &ServiceAssistant<Self>) -> f32 {
        self.tax_rate * message.0
    }
}

fn main() {
    let sys = Acteur::new();

    let taxes = sys
        .call_service_sync::<EmployeeTaxesCalculator, _>(EmployeeSalaryChange(55000.0))
        .unwrap();

    println!("Employee taxes are: {:?}", taxes);

    sys.stop();

    sys.wait_until_stopped();
}
