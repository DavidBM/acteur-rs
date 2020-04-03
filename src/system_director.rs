use std::any::TypeId;
use crate::actors::proxy::ActorReport;
use crate::actors::actor::Actor;
use crate::actors::handle::Receive;
use crate::actors::handle::Respond;
use crate::actors::director::ActorsDirector;
use std::{
    fmt::Debug,
};

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
        _actor_id: A::Id,
        _message: M,
    ) {}

    pub async fn call_to_actor<A: Actor + Respond<M>, M: Debug + Send + 'static>(
        &self,
        _actor_id: A::Id,
        _message: M,
    ) -> Result<<A as Respond<M>>::Response, &str> {
        Err("Shit")
    }

    pub(crate) async fn wait_until_stopped(&self) {}
    pub(crate) async fn stop(&self) {}
    pub(crate) fn get_statistics(&self) -> Vec<(TypeId, Vec<ActorReport>)> {
        vec!()
    }
}

impl Clone for SystemDirector {
    fn clone(&self) -> Self {
        SystemDirector {
            actor_director: self.actor_director.clone(),
        }
    }
}
