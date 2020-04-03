use crate::actors::actor::Actor;
use crate::actors::director::ActorsDirector;
use crate::actors::handle::Receive;
use crate::actors::handle::Respond;
use crate::actors::proxy::ActorReport;
use std::any::TypeId;
use std::fmt::Debug;

#[derive(Debug)]
pub(crate) struct SystemDirector {
    actor_director: ActorsDirector,
}

impl SystemDirector {
    pub(crate) fn new() -> SystemDirector {
        SystemDirector {
            actor_director: ActorsDirector::new(),
        }
    }

    pub async fn send_to_actor<A: Actor + Receive<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) {
        self.actor_director.send::<A, M>(actor_id, message).await
    }

    pub async fn call_to_actor<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        actor_id: A::Id,
        message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        self.actor_director.call::<A, M>(actor_id, message).await
    }

    pub(crate) async fn wait_until_stopped(&self) {
        self.actor_director.wait_until_stopped().await
    }

    pub(crate) async fn stop(&self) {
        self.actor_director.stop().await
    }

    pub(crate) fn get_statistics(&self) -> Vec<(TypeId, Vec<ActorReport>)> {
        self.actor_director.get_statistics()
    }
}

impl Clone for SystemDirector {
    fn clone(&self) -> Self {
        SystemDirector {
            actor_director: self.actor_director.clone(),
        }
    }
}
