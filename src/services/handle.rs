use crate::services::service::Service;
use crate::services::system_facade::SystemAssistant;
use async_trait::async_trait;
use std::fmt::Debug;

/// This Trait allow Services to receive messages.
///
/// If you want to respond to messages, use the [Serve trait](./trait.Serve.html).
///
/// This trait is compatible with [Serve trait](./trait.Serve.html) as you can implement, for the same message,
/// both traits. This trait will be executed when using the "call" or "call_sync" method from Acteur or the "call"
/// method from Assistant.
///
/// ```rust,no-run
/// use acteur::{Acteur, Service, Notify, SystemAssistant, ServiceConfiguration};
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
///     async fn initialize(system: &SystemAssistant<Self>) -> (Self, ServiceConfiguration) {
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
/// impl Notify<EmployeeHired> for EmployeeExpensesCalculator {
///     async fn handle(&self, message: EmployeeHired, _: &SystemAssistant<Self>) {
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
pub trait Notify<M: Debug>
where
    Self: Service,
{
    async fn handle(&self, message: M, system: &SystemAssistant<Self>);
}

/// This Trait allow Services to receive messages and, additionally, respond to them.
///
/// If you don't need to respond messages, use the [Notify trait](./trait.Notify.html).
///
/// This trait is compatible with [Notify trait](./trait.Notify.html) as you can implement, for the same message,
/// both traits. This trait will be executed when using the "notify" or "notify_sync" method from Acteur or the "notify"
/// method from Assistant
///
/// ```rust,no-run
/// use acteur::{Acteur, Service, Serve, ServiceConfiguration, SystemAssistant};
///
/// #[derive(Debug)]
/// struct EmployeeTaxesCalculator {
///     tax_rate: f32,
/// }
///
/// #[async_trait::async_trait]
/// impl Service for EmployeeTaxesCalculator {
///     async fn initialize(system: &SystemAssistant<Self>) -> (Self, ServiceConfiguration) {
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
///     async fn handle(&self, message: EmployeeSalaryChange, _: &SystemAssistant<Self>) -> f32 {
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

    async fn handle(&self, message: M, system: &SystemAssistant<Self>) -> Self::Response;
}
