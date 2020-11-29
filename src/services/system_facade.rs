use crate::services::broker::MessageBroker;
use crate::services::handle::{Listen, Serve};
use crate::services::service::Service;
use crate::system_director::SystemDirector;
use crate::{Actor, Receive, Respond};
use async_std::task;
use std::fmt::Debug;
use std::marker::PhantomData;

/// This object is provided to the handle method in the [Receive](./trait.Receive.html) trait for each message
/// that an Actor receives. The Actor's assistant allows to send messages and to execute some task over the system.
///
/// ```rust,no_run
/// use acteur::{Acteur, Service, Listen, ServiceConfiguration, ServiceAssistant};
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
/// struct EmployeeSalaryChange(u32);
///
/// #[async_trait::async_trait]
/// impl Listen<EmployeeSalaryChange> for EmployeeTaxesCalculator {
///
///     async fn handle(&self, message: EmployeeSalaryChange, system: &ServiceAssistant<Self>) {
///         system.stop_system();
///     }
/// }
/// ```
///
pub struct ServiceAssistant<S: Service> {
    system_director: SystemDirector,
    broker: MessageBroker,
    phantom_system: PhantomData<S>,
}

impl<S: Service> ServiceAssistant<S> {
    pub(crate) fn new(
        system_director: SystemDirector,
        broker: MessageBroker,
    ) -> ServiceAssistant<S> {
        ServiceAssistant {
            system_director,
            broker,
            phantom_system: PhantomData,
        }
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

    /// Sends a message to all actors of a type, independently of the ID.
    /// It will only send messages to actors already in Ram (already loaded)
    pub async fn send_to_all_actors<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) {
        self.system_director
            .send_to_all_actors::<A, M>(message)
            .await
    }

    /// Schedules to sends a message to the Actor with the specified Id.
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn schedule_send_to_actor<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        duration: std::time::Duration,
        message: M,
    ) {
        self.system_director
            .schedule_send_to_actor::<A, M>(actor_id, duration, message)
            .await
    }

    /// Schedules to sends a message to all actor of a type, independently of the ID.
    /// It will only send messages to actors already in Ram (already loaded)
    pub async fn schedule_send_to_all_actors<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        duration: std::time::Duration,
        message: M,
    ) {
        self.system_director
            .schedule_send_to_all_actors::<A, M>(duration, message)
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
    pub async fn send_to_service<S1: Service + Listen<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) {
        self.system_director.send_to_service::<S1, M>(message).await
    }

    /// Sends a message to a Service and waits for its response.
    /// If the Service is not loaded, it will load the service before, calling its method `initialize`
    pub async fn call_service<S1: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<S1 as Serve<M>>::Response, &str> {
        self.system_director.call_service::<S1, M>(message).await
    }

    /// Send an stop message to all actors in the system.
    /// Actors will process all the enqued messages before stop
    pub fn stop_system(&self) {
        let system = self.system_director.clone();

        task::spawn(async move {
            system.stop().await;
        });
    }

    pub async fn subscribe<M: Sync + Send + Debug + 'static>(&self)
    where
        S: Service + Listen<M>,
    {
        self.broker.register::<S, M>();
    }
}

impl<S: Service> Clone for ServiceAssistant<S> {
    fn clone(&self) -> Self {
        ServiceAssistant {
            system_director: self.system_director.clone(),
            broker: self.broker.clone(),
            phantom_system: PhantomData,
        }
    }
}

impl<S: Service> Debug for ServiceAssistant<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ServiceAssistant Facade for Service")
    }
}
