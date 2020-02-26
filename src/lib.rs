use async_std::sync::{channel, Receiver, Sender};
use async_std::task::block_on;
use async_std::task::spawn;
use async_trait::async_trait;
use dashmap::DashMap;
use std::any::Any;
use std::any::TypeId;
use std::cmp::Eq;
use std::fmt::Debug;
use std::hash::Hash;

#[async_trait]
pub trait Actor: Sized + Debug {
    type Id: Default + Eq + Hash + Send + Clone + Debug;

    async fn activate(id: Self::Id) -> Self;

    async fn deactivate(&mut self) -> Result<(), ()> {
        Ok(())
    }
}

#[async_trait]
trait Handle<T: Debug>: Actor {
    async fn handle(&mut self, message: T);
}

#[derive(Debug)]
struct TestMessage {
    pub field: String,
}

#[derive(Debug)]
struct TestActor {
    id: String,
    values: std::collections::HashMap<u32, u32>,
}

#[async_trait]
impl Actor for TestActor {
    type Id = String;

    async fn activate(id: Self::Id) -> Self {
        TestActor {
            id: id.to_string(),
            values: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl Handle<TestMessage> for TestActor {
    async fn handle(&mut self, message: TestMessage) {
        //println!("{:?}", message.field);
        self.id = message.field;
    }
}

struct System {
    actor_managers: DashMap<TypeId, Box<dyn Any>>,
}

impl System {
    pub async fn send<A, M>(&self, id: A::Id, message: M)
    where
        A: 'static + Send + Actor + Handle<M>,
        M: 'static + Send + Debug,
    {
        let type_id = TypeId::of::<A>();

        let mut manager = match self.actor_managers.get_mut(&type_id) {
            Some(manager) => manager,
            None => {
                self.add::<A>(id.clone()).await;
                match self.actor_managers.get_mut(&type_id) {
                    Some(manager) => manager,
                    None => unreachable!(),
                }
            }
        };

        match manager.downcast_mut::<ActorsManager<A>>() {
            Some(manager) => manager.send(id, message).await,
            None => unreachable!(),
        };
    }

    async fn add<A: 'static + Send + Actor>(&self, id: A::Id) {
        let type_id = TypeId::of::<A>();

        let mut manager = ActorsManager::<A>::new().await;

        manager.add(id).await;

        self.actor_managers.insert(type_id, Box::new(manager));
    }
}

pub fn start() {
    block_on(async {
        let mut manager = ActorsManager::<TestActor>::new().await;

        let message = TestMessage {
            field: "adios".to_string(),
        };

        manager.send(42.to_string(), message).await;

        let sys = System {
            actor_managers: DashMap::new(),
        };

        for _ in 0..10_000_000 {
            let message = TestMessage {
                field: "hola".to_string(),
            };

            sys.send::<TestActor, TestMessage>(43.to_string(), message)
                .await;
        }
    });
}

#[derive(Debug)]
struct ActorsManager<A: Actor> {
    actors: DashMap<A::Id, ActorProxy<A>>,
}

impl<A: 'static + Send + Actor> ActorsManager<A> {
    pub async fn new() -> ActorsManager<A> {
        ActorsManager {
            actors: DashMap::new(),
        }
    }

    pub async fn add(&mut self, id: A::Id) {
        match self.actors.get_mut(&id) {
            Some(_) => (),
            None => {
                let actor = ActorProxy::<A>::new(id.clone()).await;
                self.actors.insert(id, actor);
            }
        }
    }

    async fn send<M: 'static>(&mut self, id: A::Id, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {

        if let Some(actor) = self.actors.get_mut(&id) {
            actor.send(message).await;
            return;
        }
        
        self.add(id.clone()).await;
        
        match self.actors.get_mut(&id) {
            Some(actor) => actor.send(message).await,
            None => unreachable!(),
        }
    }
}

#[async_trait]
pub trait Envelope: Send + Debug {
    type Actor: Actor;

    async fn dispatch(&mut self, actor: &mut Self::Actor);
}

#[derive(Debug)]
struct Letter<A: Actor, M: Debug> {
    pub actor_id: A::Id,
    message: Option<M>,
}

impl<A: 'static + Handle<M> + Actor, M: Debug> Letter<A, M> {
    pub fn new(actor_id: A::Id, message: M) -> Self
    where
        A: Handle<M>,
    {
        Letter {
            actor_id,
            message: Some(message),
        }
    }

    pub async fn dispatch(&mut self, actor: &mut A) {
        match self.message.take() {
            Some(message) => {
                <A as Handle<M>>::handle(actor, message).await;
            }
            None => (),
        };
    }
}

#[async_trait]
impl<A: Send + 'static + Actor + Handle<M>, M: Send + Debug> Envelope for Letter<A, M> {
    type Actor = A;

    async fn dispatch(&mut self, actor: &mut A) {
        Letter::<A, M>::dispatch(self, actor).await
    }
}

// We may need to create a queue by actor as if not there is no way to guarantee that they actor is executed only one time at once. And even if it can be guarantee, it may not be representable.

#[derive(Debug)]
enum ActorProxyCommand<A: Actor> {
    Dispatch(Box<dyn Envelope<Actor = A>>),
    End,
}

#[derive(Debug)]
struct ActorProxy<A: Actor> {
    sender: Sender<ActorProxyCommand<A>>,
}

impl<A: 'static + Send + Actor> ActorProxy<A> {
    pub async fn new(id: <A as Actor>::Id) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let mut actor = A::activate(id).await;

        spawn(async move {
            while let Some(command) = receiver.recv().await {
                match command {
                    ActorProxyCommand::End => break,
                    ActorProxyCommand::Dispatch(mut envelope) => {
                        envelope.dispatch(&mut actor).await
                    }
                }
            }
        });

        ActorProxy { sender }
    }

    async fn send<M: 'static>(&self, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {
        let message = Letter::<A, M>::new(Default::default(), message);

        self.sender
            .send(ActorProxyCommand::Dispatch(Box::new(message)))
            .await;
    }
}
