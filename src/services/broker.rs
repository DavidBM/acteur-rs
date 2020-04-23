use crate::services::director::ServicesDirector;
use crate::services::handle::Listen;
use crate::services::service::Service;
use async_std::sync::Arc;
use dashmap::{mapref::entry::Entry, DashMap};
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct MessageBroker {
    managers: Arc<DashMap<TypeId, Vec<Box<dyn ServiceManagerPublish>>>>,
    system_director: Box<ServicesDirector>,
}

impl MessageBroker {
    pub(crate) fn new(system_director: ServicesDirector) -> MessageBroker {
        MessageBroker {
            managers: Arc::new(DashMap::new()),
            system_director: Box::new(system_director),
        }
    }

    pub(crate) fn register<S: Service + Listen<M>, M: Sync + Send + Debug + 'static>(&self) {
        let type_id = TypeId::of::<M>();

        let managers_entry = self.managers.entry(type_id);

        match managers_entry {
            Entry::Occupied(mut entry) => {
                entry
                    .get_mut()
                    .push(Box::new(ServiceManagerWrapper::<S, M>::new()));
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![Box::new(ServiceManagerWrapper::<S, M>::new())]);
            }
        };
    }

    pub(crate) async fn publish<M: Send + Clone + 'static>(&self, message: M) {
        let type_id = TypeId::of::<M>();

        if let Some(mut managers) = self.managers.get_mut(&type_id) {
            for manager in managers.value_mut() {
                let message = message.clone();
                manager.send(Box::new(message), &self.system_director).await;
            }
        }
    }
}

#[async_trait::async_trait]
trait ServiceManagerPublish: Send + Sync {
    async fn send(&mut self, message: Box<(dyn Any + Send)>, system_director: &ServicesDirector);
}

#[derive(Debug)]
struct ServiceManagerWrapper<S: Service + Listen<M>, M: Debug> {
    phantom_service: PhantomData<S>,
    phantom_message: PhantomData<M>,
}

impl<S: Service + Listen<M>, M: Debug + Send + 'static> ServiceManagerWrapper<S, M> {
    fn new() -> ServiceManagerWrapper<S, M> {
        ServiceManagerWrapper {
            phantom_service: PhantomData,
            phantom_message: PhantomData,
        }
    }

    async fn send(&mut self, message: M, system_director: &ServicesDirector) {
        system_director.send::<S, M>(message).await;
    }
}

#[async_trait::async_trait]
impl<S: Service + Listen<M>, M: Debug + Send + Sync + 'static> ServiceManagerPublish
    for ServiceManagerWrapper<S, M>
{
    async fn send(&mut self, message: Box<(dyn Any + Send)>, system_director: &ServicesDirector) {
        match message.downcast::<M>() {
            Ok(message) => {
                ServiceManagerWrapper::send(self, *message, system_director).await;
            }
            Err(_) => unreachable!(),
        }
    }
}

impl Debug for dyn ServiceManagerPublish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ServiceManagerPublish ()")
    }
}
