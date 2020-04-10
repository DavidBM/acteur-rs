use std::fmt::Debug;

///
/// Services are id-less actors that can process messages with certain concurrency.
/// The concurrency level can be configured during the "initialize" method with the
/// ServiceConfiguration struct.
///
/// ```rust,no-run
/// use acteur::{Acteur, Service, Serve, System, ServiceConfiguration};
///
/// #[derive(Debug)]
/// struct EmployeeTaxesCalculator {
///     tax_rate: f32,
/// }
///
/// #[async_trait::async_trait]
/// impl Service for EmployeeTaxesCalculator {
///     async fn initialize() -> (Self, ServiceConfiguration) {
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
///     async fn handle(&self, message: EmployeeSalaryChange, assistant: &System) -> f32 {
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
    async fn initialize() -> (Self, ServiceConfiguration);
}

#[derive(Debug)]
pub enum ServiceConcurrency {
    Automatic,
    None,
    OnePerCore,
    OneEachTwoCore,
    Fixed(usize),
}

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
