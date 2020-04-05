use async_std::sync::Arc;
use crate::actors::actor::Actor;
use crate::actors::director::ActorsDirector;
use crate::actors::handle::Receive;
use crate::actors::handle::Respond;
use crate::actors::proxy::ActorReport;
use crate::services::director::ServicesDirector;
use crate::services::handle::Notify;
use crate::services::handle::Serve;
use crate::services::service::Service;
use futures::join;
use std::any::TypeId;
use std::fmt::Debug;

#[derive(Debug)]
pub(crate) struct SystemDirector {
    actors_director: Arc<ActorsDirector>,
    services_director: Arc<ServicesDirector>,
}

impl SystemDirector {
    pub(crate) fn new() -> SystemDirector {
        let mut actors_director = Arc::new(ActorsDirector::new());
        let mut services_director = Arc::new(ServicesDirector::new());

        let system = SystemDirector {
            actors_director: actors_director.clone(),
            services_director: services_director.clone(),
        };

        if let Some(value) = Arc::get_mut(&mut actors_director) {
            value.set_system(system.clone());
        }

        if let Some(value) = Arc::get_mut(&mut services_director) {
            value.set_system(system.clone());
        }

        system
    }

    pub async fn send_to_actor<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        self.actors_director.send::<A, M>(actor_id, message).await
    }

    pub async fn call_to_actor<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        self.actors_director.call::<A, M>(actor_id, message).await
    }

    pub async fn send_to_service<S: Service + Notify<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) {
        self.services_director.send::<S, M>(message).await
    }

    pub async fn call_to_service<S: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<S as Serve<M>>::Response, &str> {
        self.services_director.call::<S, M>(message).await
    }

    pub(crate) async fn wait_until_stopped(&self) {
        let actors_future = self.actors_director.wait_until_stopped();
        let services_future = self.services_director.wait_until_stopped();

        join!(actors_future, services_future);
    }

    pub(crate) async fn stop(&self) {
        join!(self.actors_director.stop(), self.services_director.stop());
    }

    pub(crate) fn get_statistics(&self) -> Vec<(TypeId, Vec<ActorReport>)> {
        self.actors_director.get_statistics()
    }
}

impl Clone for SystemDirector {
    fn clone(&self) -> Self {
        SystemDirector {
            actors_director: self.actors_director.clone(),
            services_director: self.services_director.clone(),
        }
    }
}
