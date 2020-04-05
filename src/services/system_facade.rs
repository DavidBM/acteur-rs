use crate::services::handle::{Notify, Serve};
use crate::services::service::Service;
use crate::system_director::SystemDirector;
use crate::{Actor, Receive, Respond};
use async_std::task;
use std::fmt::Debug;

/// This object is provided to the handle method in the [Receive](./trait.Receive.html) trait for each message
/// that an Actor receives. The Actor's assistant allows to send messages and to execute some task over the system.
///
/// ```rust,no_run
/// # use acteur::{Actor, Receive, System, Acteur};
/// # use async_trait::async_trait;
/// #
/// # #[derive(Debug)]
/// # struct Employee {
/// #     salary: u32,
/// #     manager_id: u32,
/// # }
/// #
/// # #[async_trait]
/// # impl Actor for Employee {
/// #     type Id = u32;
/// #
/// #     async fn activate(_: Self::Id) -> Self {
/// #         Employee {
/// #             salary: 0, // Load from DB or set a default,
/// #             manager_id: 0 ,
/// #         }
/// #     }
/// # }
/// #
/// # #[derive(Debug)]
/// # struct Manager;
/// #
/// # #[async_trait]
/// # impl Actor for Manager {
/// #     type Id = u32;
/// #
/// #     async fn activate(_: Self::Id) -> Self {
/// #         Manager
/// #     }
/// # }
/// # #[async_trait]
/// # impl Receive<SayByeForever> for Manager {
/// #     async fn handle(&mut self, message: SayByeForever, assistant: &System<Manager>) {}
/// # }
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[derive(Debug)]
/// struct SayByeForever(String);
///
/// #[async_trait]
/// impl Receive<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, assistant: &System<Employee>) {
///         if self.salary > message.0 {
///             assistant.send::<Manager, SayByeForever>(self.manager_id, SayByeForever("Betrayer!".to_string()));
///         }
///         
///         self.salary = message.0;
///     }
/// }
///
/// # fn main() {
/// #     let sys = Acteur::new();
/// #
/// #     sys.send_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
/// #
/// #     sys.wait_until_stopped();
/// # }
///
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
    pub async fn send<A: Actor + Receive<M>, M: Debug + Send + 'static>(
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
    pub async fn call<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        self.system_director
            .call_to_actor::<A, M>(actor_id, message)
            .await
    }

    /// Sends a message to a Service.
    /// If the Service is not loaded, it will load the service before, calling its method `initialize`
    pub async fn notify<S: Service + Notify<M>, M: Debug + Send + 'static>(&self, message: M) {
        self.system_director.send_to_service::<S, M>(message).await
    }

    /// Sends a message to a Service and waits for its response.
    /// If the Service is not loaded, it will load the service before, calling its method `initialize`
    pub async fn request<S: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<S as Serve<M>>::Response, &str> {
        self.system_director.call_to_service::<S, M>(message).await
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
