use crate::actor_proxy::{ActorProxy, ActorReport};
use crate::system_director::{ActorManagerReport, SystemDirector};
use crate::envelope::ManagerEnvelope;
use crate::Actor;
use async_std::{
    sync::{channel, Arc, Receiver, Sender},
    task,
};
use dashmap::DashMap;
use std::any::TypeId;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

pub(crate) trait Manager: Send + Sync + Debug {
    fn end(&self);
    fn get_type_id(&self) -> TypeId;
    fn get_statistics(&self) -> ActorsManagerReport;
}

#[derive(Debug)]
pub(crate) enum ActorManagerProxyCommand<A: Actor> {
    Dispatch(Box<dyn ManagerEnvelope<Actor = A>>),
    EndActor(A::Id),
    EndOldActors(Duration),
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
        let mut report = vec!();

        for actor in self.actors.iter() {
            report.push(actor.get_report());
        }

        report
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
                    actor.end().await;
                    return;
                }
            }
            ActorManagerProxyCommand::EndOldActors(clean_duration) => {
                process_end_old_actors_command(&actors, clean_duration).await;
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

async fn process_end_old_actors_command<'a, A: Actor>(
    actors: &'a Arc<DashMap<A::Id, ActorProxy<A>>>,
    clean_duration: Duration,
) {
    let now = SystemTime::now();
    //let mut ended_actors = 0;
    for (index, actor) in actors.iter().enumerate() {
        let last_message = actor.get_last_sent_message_time();

        match now.duration_since(last_message) {
            Ok(dur) if dur >= clean_duration => {
                //ended_actors += 1;
                actor.end().await;
            }
            Err(_) => unreachable!(),
            _ => (),
        }

        // In the case we have many many actors, we don't want to block for long
        // Maybe it is premature optimization, but I feel that having 100K+ actors
        // and iterating in all of them can take long enough to have a 1 sec pause
        if index % 1000 == 0 {
            task::yield_now().await;
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
    match recv_until_not_end_command(receiver.clone()).await {
        None => {
            for actor in actors.iter() {
                actor.end().await;
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

async fn recv_until_not_end_command<A: Actor>(
    receiver: Receiver<ActorManagerProxyCommand<A>>,
) -> Option<ActorManagerProxyCommand<A>> {
    if receiver.is_empty() {
        return None;
    }

    while let Some(command) = receiver.recv().await {
        match command {
            ActorManagerProxyCommand::End => {
                if receiver.is_empty() {
                    return None;
                } else {
                    continue;
                }
            }
            _ => return Some(command),
        }
    }

    None
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

    fn get_type_id(&self) -> TypeId{
        ActorsManager::<A>::get_type_id(self)
    }

    fn get_statistics(&self) -> ActorsManagerReport{
        ActorsManager::<A>::get_statistics(self)
    }

}
