use crate::actors::director::ActorsDirector;
use crate::services::handle::{Listen, Serve};
use crate::services::service::Service;
use crate::system_director::SystemDirector;
use crate::{Actor, Receive, Respond};
use async_std::task;
use std::fmt::Debug;

/// This object is provided to the handle method in [Receive](./trait.Receive.html) and [Respond](./trait.Respond.html)
/// traits for each message that an Actor receives.
///
/// The Actor's assistant allows to send messages and to execute some task over the system.
///
/// ```rust,no_run
/// # use acteur::{Actor, Receive, ActorAssistant, Acteur};
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
/// #     async fn activate(_: Self::Id, _: &ActorAssistant<Self>) -> Self {
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
/// #     async fn activate(_: Self::Id, _: &ActorAssistant<Self>) -> Self {
/// #         Manager
/// #     }
/// # }
/// # #[async_trait]
/// # impl Receive<SayByeForever> for Manager {
/// #     async fn handle(&mut self, message: SayByeForever, assistant: &ActorAssistant<Manager>) {}
/// # }
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[derive(Debug)]
/// struct SayByeForever(String);
///
/// #[async_trait]
/// impl Receive<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, assistant: &ActorAssistant<Employee>) {
///         if self.salary > message.0 {
///             assistant.send_to_actor::<Manager, SayByeForever>(self.manager_id, SayByeForever("Betrayer!".to_string()));
///         }
///         
///         self.salary = message.0;
///     }
/// }
///
/// # fn main() {
/// #     let sys = Acteur::new();
/// #
/// #     sys.send_to_actor_sync::<Employee, SalaryChanged>(42, SalaryChanged(55000));
/// #
/// #     sys.wait_until_stopped();
/// # }
///
/// ```
///
pub struct ActorAssistant<A: Actor> {
    system_director: SystemDirector,
    actors_director: ActorsDirector,
    actor_id: A::Id,
}

impl<A: Actor> ActorAssistant<A> {
    pub(crate) fn new(
        system_director: SystemDirector,
        actors_director: ActorsDirector,
        actor_id: A::Id,
    ) -> ActorAssistant<A> {
        ActorAssistant {
            system_director,
            actors_director,
            actor_id,
        }
    }

    /// Sends a message to the Actor with the specified Id.
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn send_to_actor<A2: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A2::Id,
        message: M,
    ) {
        self.actors_director.send::<A2, M>(actor_id, message).await
    }

    /// Sends a message to all actors independently of the ID.
    /// It will only send messages to actors already in Ram (already loaded)
    pub async fn send_to_all_actors<A2: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) {
        self.actors_director.send_to_all::<A2, M>(message).await
    }

    /// Schedules to sends a message to the Actor with the specified Id.
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn schedule_send_to_actor<A2: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A2::Id,
        duration: std::time::Duration,
        message: M,
    ) {
        self.system_director
            .schedule_send_to_actor::<A2, M>(actor_id, duration, message)
            .await
    }

    /// Schedules to sends a message to all actors independently of the ID.
    /// It will only send messages to actors already in Ram (already loaded)
    pub async fn schedule_send_to_all_actors<A2: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        duration: std::time::Duration,
        message: M,
    ) {
        self.system_director
            .schedule_send_to_all_actors::<A2, M>(duration, message)
            .await
    }

    /// Sends a message to the Actor with the specified Id and waits the actor's response .
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn call_actor<A2: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A2::Id,
        message: M,
    ) -> Result<<A2 as Respond<M>>::Response, &str> {
        self.actors_director.call::<A2, M>(actor_id, message).await
    }

    /// Sends a message to a Service.
    /// If the Service is not loaded, it will load the service before, calling its method `initialize`
    pub async fn send_to_service<S: Service + Listen<M>, M: Debug + Send + 'static>(
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

    /// Enqueues a end command in the Actor messages queue. The actor will consume all mesages before ending.
    /// Keep in mind that event is an actor is stopped, a new message in the future can wake up the actor.
    pub async fn stop(&self) {
        self.actors_director
            .stop_actor::<A>(self.actor_id.clone())
            .await;
    }

    /// Returns the Actor's Id defined by the [`Actor`](./trait.Actor.html) Trait
    pub async fn get_id(&self) -> &A::Id {
        &self.actor_id
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

impl<A: Actor> Clone for ActorAssistant<A> {
    fn clone(&self) -> Self {
        ActorAssistant {
            actors_director: self.actors_director.clone(),
            actor_id: self.actor_id.clone(),
            system_director: self.system_director.clone(),
        }
    }
}

impl<A: Actor> Debug for ActorAssistant<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActorSecretary for {}", std::any::type_name::<A>())
    }
}
