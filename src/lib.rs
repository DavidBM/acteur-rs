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
use std::sync::Arc;

#[async_trait]
pub trait Actor: Sized + Debug {
    type Id: Default + Eq + Hash + Send + Sync + Clone + Debug;

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
    pub async fn send<A: 'static + Actor + Handle<M>, M: Debug>(&self, id: A::Id, message: M) {
        let type_id = TypeId::of::<A>();

        let mut actor = match self.actor_managers.get_mut(&type_id) {
            Some(actor) => actor,
            None => {
                self.add::<A>(id).await;
                match self.actor_managers.get_mut(&type_id) {
                    Some(actor) => actor,
                    None => unreachable!(),
                }
            }
        };

        match actor.downcast_mut::<A>() {
            Some(actor) => actor.handle(message).await,
            None => unreachable!(),
        };
    }

    async fn add<A: 'static + Actor>(&self, id: A::Id) {
        let type_id = TypeId::of::<A>();

        let actor = A::activate(id).await;

        self.actor_managers.insert(type_id, Box::new(actor));
    }
}

pub fn start() {
    block_on(async {
        let manager = ActorsManager::<TestActor>::new(42.to_string()).await;

        let message = TestMessage {
            field: "adios".to_string(),
        };

        manager.send(message).await;

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
    actors: Arc<DashMap<A::Id, A>>,
    sender: Sender<Box<dyn Envelope<Actor = A>>>,
}

impl<A: Sync + 'static + Send + Actor> ActorsManager<A> {
    pub async fn new(id: <A as Actor>::Id) -> ActorsManager<A> {
        let (sender, receiver): (
            Sender<Box<dyn Envelope<Actor = A>>>,
            Receiver<Box<dyn Envelope<Actor = A>>>,
        ) = channel(5);

        let actor = A::activate(Default::default()).await;

        let actors_original = Arc::new(DashMap::new());

        let id2 = id.clone();

        actors_original.insert(id2, actor);

        let actors = actors_original.clone();
        spawn(async move {
            while let Some(mut envelope) = receiver.recv().await {
                match actors.get_mut(&id) {
                    Some(mut actor) => envelope.dispatch(&mut actor).await,
                    None => (),
                }
            }
        });

        ActorsManager {
            actors: actors_original,
            sender,
        }
    }

    async fn send<M: 'static>(&self, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {
        let message = Letter::<A, M>::new(Default::default(), message);

        self.sender.send(Box::new(message)).await;
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
impl<A: Send + 'static + Actor + Handle<M>, M: Send + Debug> Envelope
    for Letter<A, M>
{
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
    sender: Sender<Box<dyn Envelope<Actor = A>>>,
}

impl<A: Sync + 'static + Send + Actor> ActorProxy<A> {
    pub async fn new(id: <A as Actor>::Id) -> ActorProxy<A> {
        let (sender, receiver): (
            Sender<Box<dyn Envelope<Actor = A>>>,
            Receiver<Box<dyn Envelope<Actor = A>>>,
        ) = channel(5);

        let mut actor = A::activate(id).await;

        spawn(async move {
            while let Some(mut envelope) = receiver.recv().await {
                envelope.dispatch(&mut actor).await
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

        self.sender.send(Box::new(message)).await;
    }
}
