use crate::services::system_facade::ServiceAssistant;
use std::fmt::Debug;

///
/// Services are id-less actors that can process messages with certain concurrency.
/// The concurrency level can be configured during the "initialize" method with the
/// ServiceConfiguration struct.
///
/// ```rust,no_run
/// use acteur::{Acteur, Service, Serve, ServiceAssistant, ServiceConfiguration};
///
/// #[derive(Debug)]
/// struct EmployeeTaxesCalculator {
///     tax_rate: f32,
/// }
///
/// #[async_trait::async_trait]
/// impl Service for EmployeeTaxesCalculator {
///     async fn initialize(system: &ServiceAssistant<Self>) -> (Self, ServiceConfiguration) {
///         let service = EmployeeTaxesCalculator {
///             tax_rate: 0.21,
///         };
///
///         let service_conf = ServiceConfiguration::default();
///
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
///
///     async fn handle(&self, message: EmployeeSalaryChange, system: &ServiceAssistant<Self>) -> f32 {
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
#[async_trait::async_trait]
pub trait Service: Sized + Send + Sync + Debug + 'static {
    async fn initialize(system: &ServiceAssistant<Self>) -> (Self, ServiceConfiguration);
}

/// Defined the concurrency from the Service.
/// 
/// In the majority of cases you may want to use "Automatic".
#[derive(Debug)]
pub enum ServiceConcurrency {
    /// This mode will check the size of your struct.
    /// 
    /// If the size is 0, it will assume Unlimited concurrency as there shouldn't be any synchronization between
    /// handler executions.
    /// 
    /// If the size is not 0, it will Asume OnePerCore
    Automatic,
    /// only one message processed at a time.
    None,
    /// sets concurrency to the same number of cores
    OnePerCore,
    /// sets concurrency to cores/2
    OneEachTwoCore,
    /// creates a fixed number of loops, efectively allowing a fixed concurrency.
    Fixed(usize),
    /// In this mode the only one loop will be spawned, but it won't await the service handler, instead it will spawn 
    /// a concurrent task that will be executed whenever is possible. 
    Unlimited,
}

/// Defines the service configuration. For now it only contains the concurrency parameters.
/// 
/// This struct implements the Default trait. You can call it with `ServiceConfiguration::default()` and 
/// it should work well for the most of the cases..
pub struct ServiceConfiguration {
    pub concurrency: ServiceConcurrency,
}

impl Default for ServiceConfiguration {
    fn default() -> ServiceConfiguration {
        ServiceConfiguration {
            concurrency: ServiceConcurrency::Automatic,
        }
    }
}
