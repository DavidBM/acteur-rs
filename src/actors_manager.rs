use crate::actor_proxy::{ActorProxy, ActorReport};
use crate::envelope::ManagerEnvelope;
use crate::system_director::{ActorManagerReport, SystemDirector};
use crate::Actor;
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task,
};
use dashmap::DashMap;
use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) trait Manager: Send + Sync + Debug {
    fn end(&self);
    fn get_type_id(&self) -> TypeId;
    fn get_statistics(&self) -> ActorsManagerReport;
    fn get_sender_as_any(&self) -> Box<dyn Any>;
    fn end_filtered(&self, filter_fn: &Box<dyn Fn(ActorReport) -> bool>);
}

#[derive(Debug)]
pub(crate) enum ActorManagerProxyCommand<A: Actor> {
    Dispatch(Box<dyn ManagerEnvelope<Actor = A>>),
    EndActor(A::Id),
    End,
}

#[derive(Debug)]
pub(crate) enum ActorProxyReport<A: Actor> {
    ActorStopped(A::Id),
}

pub(crate) type ActorsManagerReport = Vec<ActorReport>;

#[derive(Debug)]
pub(crate) struct ActorsManager<A: Actor> {
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    sender: Sender<ActorManagerProxyCommand<A>>,
}

impl<A: Actor> ActorsManager<A> {
    pub fn new(
        system_director: SystemDirector,
        manager_report_sender: Sender<ActorManagerReport>,
    ) -> ActorsManager<A> {
        // Channel in order to receive commands (like sending messages to actors, stopping, etc)
        let (sender, receiver) = channel::<ActorManagerProxyCommand<A>>(150_000);

        // Channel for getting reports back from ActorProxies
        let (report_sender, report_receiver) = channel::<ActorProxyReport<A>>(1);

        let actors = Arc::new(DashMap::new());
        let is_ending = Arc::new(AtomicBool::new(false));

        // Loop for processing commands
        task::spawn(actor_manager_loop(
            receiver,
            sender.clone(),
            actors.clone(),
            system_director,
            report_sender,
            is_ending.clone(),
        ));

        // Loop for processing ActorProxies reports
        task::spawn(actor_proxy_report_loop(
            report_receiver,
            actors.clone(),
            manager_report_sender,
            is_ending,
        ));

        ActorsManager { actors, sender }
    }

    pub(crate) fn end(&self) {
        let sender = self.sender.clone();
        task::spawn(async move {
            sender.send(ActorManagerProxyCommand::End).await;
        });
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

    pub(crate) fn end_filtered(&self, filter_fn: &Box<dyn Fn(ActorReport) -> bool>) {
        for actor in self.actors.iter() {
            if !filter_fn(actor.get_report()) {
                actor.end();
            }
        }
    }
}

#[derive(Debug)]
enum LoopStatus {
    Continue,
    Stop,
}

async fn actor_manager_loop<A: Actor>(
    receiver: Receiver<ActorManagerProxyCommand<A>>,
    sender: Sender<ActorManagerProxyCommand<A>>,
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    system_director: SystemDirector,
    report_sender: Sender<ActorProxyReport<A>>,
    is_ending: Arc<AtomicBool>,
) {
    while let Some(command) = receiver.recv().await {
        match command {
            ActorManagerProxyCommand::Dispatch(command) => {
                process_dispatch_command(command, &actors, &system_director, &report_sender).await;
            }
            ActorManagerProxyCommand::EndActor(id) => {
                if let Some(actor) = actors.get_mut(&id) {
                    actor.end();
                    return;
                }
            }
            ActorManagerProxyCommand::End => {
                let loop_continuity = process_end_command(
                    &receiver,
                    &actors,
                    &sender,
                    &system_director,
                    &report_sender,
                    &is_ending,
                )
                .await;

                if let LoopStatus::Stop = loop_continuity {
                    break;
                }
            }
        }
    }
}

async fn actor_proxy_report_loop<A: Actor>(
    receiver: Receiver<ActorProxyReport<A>>,
    actors: Arc<DashMap<A::Id, ActorProxy<A>>>,
    system_report: Sender<ActorManagerReport>,
    is_ending: Arc<AtomicBool>,
) {
    while let Some(command) = receiver.recv().await {
        match command {
            ActorProxyReport::ActorStopped(id) => {
                actors.remove(&id);

                // We only send the "end" report in the case where an End command was previously sent
                if actors.is_empty() && is_ending.load(Ordering::Relaxed) {
                    system_report
                        .send(ActorManagerReport::ManagerEnded(TypeId::of::<A>()))
                        .await;
                    break;
                }
            }
        }
    }
}

async fn process_end_command<'a, A: Actor>(
    receiver: &'a Receiver<ActorManagerProxyCommand<A>>,
    actors: &'a Arc<DashMap<A::Id, ActorProxy<A>>>,
    sender: &'a Sender<ActorManagerProxyCommand<A>>,
    system_director: &'a SystemDirector,
    report_sender: &'a Sender<ActorProxyReport<A>>,
    is_ending: &'a Arc<AtomicBool>,
) -> LoopStatus {
    is_ending.store(true, Ordering::Relaxed);

    // We may find cases where we can have several End command in a row. In that case,
    // we want to consume all the end command together until we totally consume the messages
    // or we find a not-end command
    match recv_until_command_or_end!(receiver, ActorManagerProxyCommand::End).await {
        None => {
            for actor in actors.iter() {
                actor.end();
            }
            LoopStatus::Stop
        }
        Some(ActorManagerProxyCommand::Dispatch(command)) => {
            // If there are any message left, we postpone the shutdown.
            sender.send(ActorManagerProxyCommand::End).await;
            process_dispatch_command(command, &actors, &system_director, &report_sender).await;
            LoopStatus::Continue
        }
        _ => unreachable!(),
    }
}

async fn process_dispatch_command<'a, A: Actor>(
    mut command: Box<dyn ManagerEnvelope<Actor = A>>,
    actors: &'a Arc<DashMap<A::Id, ActorProxy<A>>>,
    system_director: &'a SystemDirector,
    report_sender: &'a Sender<ActorProxyReport<A>>,
) {
    let actor_id = command.get_actor_id();

    if let Some(mut actor) = actors.get_mut(&actor_id) {
        command.deliver(&mut actor).await;
        return;
    }

    let mut actor = ActorProxy::<A>::new(
        system_director.clone(),
        actor_id.clone(),
        report_sender.clone(),
    );

    command.deliver(&mut actor).await;

    actors.insert(actor_id, actor);
}

impl<A: Actor> Clone for ActorsManager<A> {
    fn clone(&self) -> Self {
        ActorsManager {
            actors: self.actors.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<A: Actor> Manager for ActorsManager<A> {
    fn end(&self) {
        ActorsManager::<A>::end(self);
    }

    fn get_type_id(&self) -> TypeId {
        ActorsManager::<A>::get_type_id(self)
    }

    fn get_statistics(&self) -> ActorsManagerReport {
        ActorsManager::<A>::get_statistics(self)
    }

    fn get_sender_as_any(&self) -> Box<dyn Any> {
        Box::new(ActorsManager::<A>::get_sender(self))
    }

    fn end_filtered(&self, filter_fn: &Box<dyn Fn(ActorReport) -> bool>) {
        ActorsManager::<A>::end_filtered(self, filter_fn);
    }
}
