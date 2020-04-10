use crate::services::handle::{Notify, Serve};
use crate::services::service::Service;
use crate::system_director::SystemDirector;
use crate::{Actor, Receive, Respond};
use async_std::task;
use std::fmt::Debug;

/// This object is provided to the handle method in the [Receive](./trait.Receive.html) trait for each message
/// that an Actor receives. The Actor's assistant allows to send messages and to execute some task over the system.
///
/// ```rust,no-run
/// use acteur::{Acteur, Service, Notify, ServiceConfiguration, System};
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
/// struct EmployeeSalaryChange(u32);
///
/// #[async_trait::async_trait]
/// impl Notify<EmployeeSalaryChange> for EmployeeTaxesCalculator {
///
///     async fn handle(&self, message: EmployeeSalaryChange, system: &System) {
///         system.stop_system();
///     }
/// }
/// ```
///
pub struct System {
    system_director: SystemDirector,
}

impl System {
    pub(crate) fn new(system_director: SystemDirector) -> System {
        System { system_director }
    }

    /// Sends a message to the Actor with the specified Id.
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn send_to_actor<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        self.system_director
            .send_to_actor::<A, M>(actor_id, message)
            .await
    }

    /// Sends a message to the Actor with the specified Id and waits the actor's response .
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn call_actor<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        self.system_director
            .call_actor::<A, M>(actor_id, message)
            .await
    }

    /// Sends a message to a Service.
    /// If the Service is not loaded, it will load the service before, calling its method `initialize`
    pub async fn send_to_service<S: Service + Notify<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) {
        self.system_director.send_to_service::<S, M>(message).await
    }

    /// Sends a message to a Service and waits for its response.
    /// If the Service is not loaded, it will load the service before, calling its method `initialize`
    pub async fn call_service<S: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<S as Serve<M>>::Response, &str> {
        self.system_director.call_service::<S, M>(message).await
    }

    /// Send an stop message to all actors in the system.
    /// Actors will process all the enqued messages before stop
    pub fn stop_system(&self) {
        let system = self.system_director.clone();

        task::spawn(async move {
            system.stop().await;
        });
    }
}

impl Clone for System {
    fn clone(&self) -> Self {
        System {
            system_director: self.system_director.clone(),
        }
    }
}

impl Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "System Facade for Service")
    }
}
