use crate::actors::proxy::{ActorProxy, ActorReport};
use crate::envelope::ManagerEnvelope;
use crate::system_director::SystemDirector;
use crate::Actor;
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task,
};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};

#[async_trait::async_trait]
pub(crate) trait Manager: Send + Sync + Debug {
    fn end(&self);
    fn get_type_id(&self) -> TypeId;
    fn get_statistics(&self) -> ActorsManagerReport;
    fn get_sender_as_any(&self) -> Box<dyn Any>;
    fn is_empty(&self) -> bool;
    fn remove_actor(&self, actor_id: Box<dyn Any + Send>);
}

#[derive(Debug)]
pub(crate) enum ActorManagerProxyCommand<A: Actor> {
    Dispatch(Box<dyn ManagerEnvelope<Actor = A>>),
    EndActor(A::Id),
}

pub(crate) type ActorsManagerReport = Vec<ActorReport>;

#[derive(Debug)]
pub(crate) struct ActorsManager<A: Actor> {
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    sender: Sender<ActorManagerProxyCommand<A>>,
    is_ending: Arc<AtomicBool>,
    system_director: SystemDirector,
}

impl<A: Actor> ActorsManager<A> {
    pub fn new(system_director: SystemDirector) -> ActorsManager<A> {
        // Channel in order to receive commands (like sending messages to actors, stopping, etc)
        let (sender, receiver) = channel::<ActorManagerProxyCommand<A>>(150_000);

        let actors = Arc::new(DashMap::new());
        let is_ending = Arc::new(AtomicBool::new(false));

        let manager = ActorsManager {
            actors: actors.clone(),
            sender,
            is_ending: is_ending.clone(),
            system_director: system_director.clone(),
        };

        // Loop for processing commands
        task::spawn(actor_manager_loop(
            receiver,
            actors,
            system_director,
            manager.clone(),
            is_ending,
        ));

        manager
    }

    pub(crate) fn end(&self) {
        self.is_ending.store(true, Ordering::Relaxed);

        for actor in self.actors.iter() {
            actor.end();
        }
    }

    pub(crate) async fn signal_actor_removed(&self) {
        // Maybe becayse it is not marked to be removed, or because there are still actors or because
        // there are still remaining messages to be sent.
        if !self.is_ready_to_be_removed() {
            return;
        }
        // If it can be removed, we block the System HashMap entry
        let entry = self
            .system_director
            .get_blocking_manager_entry(std::any::TypeId::of::<A>());

        // Double check that any more messages were received during the preivous line
        // and that no new actors were created.
        if !self.is_ready_to_be_removed() {
            return;
        }

        if let Entry::Occupied(entry) = entry {
            // Remove the entry from the system HashMap
            entry.remove();
            // Signaling only when we really remove the manager.
            self.system_director.signal_manager_removed().await;
        }
    }

    /// A manager is ready to be removed only if there are no more messages pending to be delivered,
    /// has no active actors and it is flagged to be ended.
    fn is_ready_to_be_removed(&self) -> bool {
        self.is_ending.load(Ordering::Acquire) && self.actors.is_empty() && self.sender.is_empty()
    }

    pub(crate) fn get_sender(&self) -> Sender<ActorManagerProxyCommand<A>> {
        self.sender.clone()
    }

    pub(crate) fn get_type_id(&self) -> TypeId {
        TypeId::of::<A>()
    }

    pub(crate) fn get_statistics(&self) -> ActorsManagerReport {
        let mut report = vec![];

        for actor in self.actors.iter() {
            report.push(actor.get_report());
        }

        report
    }

    pub(crate) fn remove_actor(&self, actor_id: A::Id) {
        self.actors.remove(&actor_id);
    }

    /// Returns the Entry of the actorProxy in the general HashMap, making not possible to send any messages
    /// until the Entry is droped.
    pub(crate) fn get_blocking_actor_entry(&self, id: A::Id) -> Entry<A::Id, ActorProxy<A>> {
        self.actors.entry(id)
    }

    fn is_empty(&self) -> bool {
        self.actors.is_empty() && self.sender.is_empty()
    }
}

async fn actor_manager_loop<A: Actor>(
    receiver: Receiver<ActorManagerProxyCommand<A>>,
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    system_director: SystemDirector,
    manager: ActorsManager<A>,
    is_ending: Arc<AtomicBool>,
) {
    while let Some(command) = receiver.recv().await {
        match command {
            ActorManagerProxyCommand::Dispatch(command) => {
                process_dispatch_command(command, &actors, &system_director, &manager, &is_ending)
                    .await;
            }
            ActorManagerProxyCommand::EndActor(actor_id) => {
                process_end_actor_command(actor_id, &actors).await;
            }
        }
    }
}

async fn process_end_actor_command<'a, A: Actor>(
    actor_id: A::Id,
    actors: &'a Arc<DashMap<A::Id, ActorProxy<A>>>,
) {
    if let Some(actor) = actors.get_mut(&actor_id) {
        actor.end();
    }
}

async fn process_dispatch_command<'a, A: Actor>(
    mut command: Box<dyn ManagerEnvelope<Actor = A>>,
    actors: &'a Arc<DashMap<A::Id, ActorProxy<A>>>,
    system_director: &'a SystemDirector,
    manager: &'a ActorsManager<A>,
    is_ending: &'a Arc<AtomicBool>,
) {
    let actor_id = command.get_actor_id();

    if let Some(mut actor) = actors.get_mut(&actor_id) {
        command.deliver(&mut actor).await;
        return;
    }

    let mut actor =
        ActorProxy::<A>::new(system_director.clone(), manager.clone(), actor_id.clone());

    command.deliver(&mut actor).await;

    if is_ending.load(Ordering::Relaxed) {
        actor.end();
    }

    actors.insert(actor_id, actor);
}

impl<A: Actor> Clone for ActorsManager<A> {
    fn clone(&self) -> Self {
        ActorsManager {
            actors: self.actors.clone(),
            sender: self.sender.clone(),
            is_ending: self.is_ending.clone(),
            system_director: self.system_director.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<A: Actor> Manager for ActorsManager<A> {
    fn end(&self) {
        ActorsManager::<A>::end(self)
    }

    fn get_type_id(&self) -> TypeId {
        ActorsManager::<A>::get_type_id(self)
    }

    fn get_statistics(&self) -> ActorsManagerReport {
        ActorsManager::<A>::get_statistics(self)
    }

    fn is_empty(&self) -> bool {
        ActorsManager::<A>::is_empty(self)
    }

    fn get_sender_as_any(&self) -> Box<dyn Any> {
        Box::new(ActorsManager::<A>::get_sender(self))
    }

    fn remove_actor(&self, actor_id: Box<dyn Any + Send>) {
        match actor_id.downcast::<A::Id>() {
            Ok(actor_id) => ActorsManager::<A>::remove_actor(self, *actor_id),
            Err(_) => (),
        }
    }
}
