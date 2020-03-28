use crate::actors_manager::ActorsManager;
use crate::system_director::SystemDirector;
use crate::{Actor, Receive};
use async_std::task;
use std::fmt::Debug;

/// This object is provided to the handle method in the [Receive](./trait.Receive.html) trait for each message 
/// that an Actor receives. The Actor's assistant allows to send messages and to execute some task over the system.
///
/// ```rust,no_run
/// # use acteur::{Actor, Receive, Assistant, Acteur};
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
/// #     async fn handle(&mut self, message: SayByeForever, assistant: &Assistant<Manager>) {}
/// # }
/// #[derive(Debug)]
/// struct SalaryChanged(u32);
///
/// #[derive(Debug)]
/// struct SayByeForever(String);
///
/// #[async_trait]
/// impl Receive<SalaryChanged> for Employee {
///     async fn handle(&mut self, message: SalaryChanged, assistant: &Assistant<Employee>) {
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
pub struct Assistant<A: Actor> {
    system_director: SystemDirector,
    actor_id: A::Id,
    manager: ActorsManager<A>,
}

impl<A: Actor> Assistant<A> {
    pub(crate) fn new(
        system_director: SystemDirector,
        manager: ActorsManager<A>,
        actor_id: A::Id,
    ) -> Assistant<A> {
        Assistant {
            system_director,
            actor_id,
            manager,
        }
    }

    /// Sends a message to the Actor with the specified Id.
    /// If the Actor is not loaded, it will load the actor before, calling its method `activate`
    pub async fn send<A2: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A2::Id,
        message: M,
    ) {
        self.system_director.send::<A2, M>(actor_id, message).await
    }

    /// Enqueues a end command in the Actor messages queue. The actor will consume all mesages before ending.
    /// Keep in mind that event is an actor is stopped, a new message in the future can wake up the actor.
    pub async fn stop(&self) {
        self.system_director
            .stop_actor::<A>(self.actor_id.clone())
            .await;
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

impl<A: Actor> Clone for Assistant<A> {
    fn clone(&self) -> Self {
        Assistant {
            system_director: self.system_director.clone(),
            actor_id: self.actor_id.clone(),
            manager: self.manager.clone(),
        }
    }
}

impl<A: Actor> Debug for Assistant<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActorSecretary for {}", std::any::type_name::<A>())
    }
}
