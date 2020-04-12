use crate::actors::actor::Actor;
use crate::actors::director::{ActorsDirector, ActorsDirectorConfiguration};
use crate::actors::handle::Receive;
use crate::actors::handle::Respond;
use crate::actors::proxy::ActorReport;
use crate::services::director::ServicesDirector;
use crate::services::handle::Notify;
use crate::services::handle::Serve;
use crate::services::service::Service;
use async_std::{sync::Arc, task::block_on};
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
        let mut actors_director = Arc::new(ActorsDirector::new(ActorsDirectorConfiguration {
            innactivity_seconds_until_actor_end: std::time::Duration::from_secs(300),
        }));

        let mut services_director = Arc::new(ServicesDirector::new());

        let system = SystemDirector {
            actors_director: actors_director.clone(),
            services_director: services_director.clone(),
        };

        let system_to_retur = system.clone();

        block_on(async move {
            Arc::make_mut(&mut actors_director)
                .set_system(system.clone())
                .await;
            Arc::make_mut(&mut services_director)
                .set_system(system.clone())
                .await;
        });

        system_to_retur
    }

    pub async fn send_to_actor<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        self.actors_director.send::<A, M>(actor_id, message).await
    }

    pub async fn call_actor<A: Actor + Respond<M>, M: Debug + Send + 'static>(
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

    pub async fn call_service<S: Service + Serve<M>, M: Debug + Send + 'static>(
        &self,
        message: M,
    ) -> Result<<S as Serve<M>>::Response, &str> {
        self.services_director.call::<S, M>(message).await
    }

    pub(crate) async fn preload_service<S: Service>(&self) {
        self.services_director.preload::<S>().await;
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

    pub(crate) async fn publish<M: Send + Clone + 'static>(&self, message: M) {
        self.services_director.publish(message).await
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
