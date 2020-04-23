use crate::services::service::Service;
use crate::services::system_facade::ServiceActorAssistant;
use async_trait::async_trait;
use std::fmt::Debug;

/// This Trait allow Services to receive messages.This is the most efficient way to process messages as it doesn't
/// require to respond the message.
///
/// If you want to respond to messages, use the [Serve trait](./trait.Serve.html).
///
/// This trait is compatible with [Serve trait](./trait.Serve.html) as you can implement, for the same message,
/// both traits. This trait will be executed when using the "call" or "call_sync" method from Acteur or the "call"
/// method from ActorAssistant.
///
/// ```rust,no-run
/// use acteur::{Acteur, Service, Listen, ServiceActorAssistant, ServiceConfiguration};
/// use async_std::sync::Mutex;
///
/// #[derive(Debug)]
/// struct EmployeeExpensesCalculator {
///     // Services have concurrency, therefore, your state needs to be under a Mutex.
///     // Highly recommended using an async Mutex instead of Rust Mutex
///     employee_expenses: Mutex<f32>,
/// }
///
/// #[async_trait::async_trait]
/// impl Service for EmployeeExpensesCalculator {
///     async fn initialize(system: &ServiceActorAssistant<Self>) -> (Self, ServiceConfiguration) {
///         let service = EmployeeExpensesCalculator {
///             employee_expenses: Mutex::new(0.0),
///         };
///         let service_conf = ServiceConfiguration::default();
///         (service, service_conf)
///     }
/// }
///
/// #[derive(Debug)]
/// struct EmployeeHired(f32);
///
/// #[async_trait::async_trait]
/// impl Listen<EmployeeHired> for EmployeeExpensesCalculator {
///     async fn handle(&self, message: EmployeeHired, _: &ServiceActorAssistant<Self>) {
///         *self.employee_expenses.lock().await += message.0;
///     }
/// }
/// fn main() {
///     let sys = Acteur::new();
///     sys.send_to_service_sync::<EmployeeExpensesCalculator, _>(EmployeeHired(55000.0));
///     sys.stop();
///     sys.wait_until_stopped();
/// }
/// ```
///
#[async_trait]
pub trait Listen<M: Debug>
where
    Self: Service,
{
    async fn handle(&self, message: M, system: &ServiceActorAssistant<Self>);
}

/// This Trait allow Services to receive messages and, additionally, respond to them.
///
/// If you don't need to respond messages, use the [Listen trait](./trait.Listen.html).
///
/// This trait is compatible with [Listen trait](./trait.Listen.html) as you can implement, for the same message,
/// both traits. This trait will be executed when using the "notify" or "notify_sync" method from Acteur or the "notify"
/// method from ActorAssistant
/// 
/// Keep in mind that if someone waits for this service to respond and this service has a long queue of messages
/// to process, the response can take long time, slowing down who is calling this service. Preffer always to use 
/// the [Serve trait](./trait.Serve.html) if you can.
///
/// ```rust,no-run
/// use acteur::{Acteur, Service, Serve, ServiceConfiguration, ServiceActorAssistant};
///
/// #[derive(Debug)]
/// struct EmployeeTaxesCalculator {
///     tax_rate: f32,
/// }
///
/// #[async_trait::async_trait]
/// impl Service for EmployeeTaxesCalculator {
///     async fn initialize(system: &ServiceActorAssistant<Self>) -> (Self, ServiceConfiguration) {
///         let service = EmployeeTaxesCalculator {
///             tax_rate: 0.21,
///         };
///         let service_conf = ServiceConfiguration::default();
///         (service, service_conf)
///     }
/// }
///
/// #[derive(Debug)]
/// struct EmployeeSalaryChange(f32);
///
/// #[async_trait::async_trait]
/// impl Serve<EmployeeSalaryChange> for EmployeeTaxesCalculator {
///     type Response = f32;
///     async fn handle(&self, message: EmployeeSalaryChange, _: &ServiceActorAssistant<Self>) -> f32 {
///         self.tax_rate * message.0
///     }
/// }
///
/// fn main() {
///     let sys = Acteur::new();
///     
///     let taxes = sys.call_service_sync::<EmployeeTaxesCalculator, _>(EmployeeSalaryChange(55000.0)).unwrap();
///
///     println!("Employee taxes are: {:?}", taxes);
///
///     sys.stop();
///     
///     sys.wait_until_stopped();
/// }
/// ```
///
#[async_trait]
pub trait Serve<M: Debug>: Sized + Service {
    type Response: Send;

    async fn handle(&self, message: M, system: &ServiceActorAssistant<Self>) -> Self::Response;
}
