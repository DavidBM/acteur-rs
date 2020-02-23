use std::any::Any;
use async_trait::{async_trait};
use async_std::task::block_on;
use std::collections::HashMap;
use std::any::TypeId;


#[async_trait]
trait Actor: Sized {
    type Id;

    pub async fn activate(id: Self::Id) -> Self;

    async fn deactivate(&mut self)  -> Result<(), ()>{
        Ok(())
    }
}

#[async_trait]
trait Handle<T>: Actor {
    async fn handle(&mut self, message: T);
}


#[derive(Debug)]
struct TestMessage {
    field: String
}


#[derive(Debug)]
struct TestActor {
    id: String
}

#[async_trait]
impl Actor for TestActor {
    type Id = u64;

    async fn activate(id: Self::Id) -> Self {
        println!("Starting TestActor with id {}", id);
        TestActor {
            id: id.to_string()
        }
    }
}

#[async_trait]
impl Handle<TestMessage> for TestActor {
    async fn handle(&mut self, message: TestMessage) {
        println!("Handling TestMessage in TestActor with id {:?}", message);
        println!("{:?}", message);
    }
}



#[derive(Debug)]
struct TestMessage2 {
    field: String
}


#[derive(Debug)]
struct TestActor2 {
    id: String
}

#[async_trait]
impl Actor for TestActor2 {
    type Id = u64;

    async fn activate(id: Self::Id) -> Self {
        TestActor2 {
            id: id.to_string()
        }
    }
}

#[async_trait]
impl Handle<TestMessage2> for TestActor2 {
    async fn handle(&mut self, message: TestMessage2) {
        println!("Handling TestMessage2 in TestActor2 with id {:?}", message);
    }
}

struct System {
    actor_managers: HashMap<TypeId, Box<dyn Any>>,
}

impl System {
    async fn send<A: 'static + Actor + Handle<M>, M>(&mut self, id: A::Id, message: M) {
        let type_id = TypeId::of::<A>();

        let actor = match self.actor_managers.get_mut(&type_id) {
            Some(actor) => actor,
            None => {
                self.add::<A>(id).await;
                match self.actor_managers.get_mut(&type_id) {
                    Some(actor) => actor,
                    None => unreachable!(),
                }
            },
        };

        match  actor.downcast_mut::<A>() {
            Some(actor) => actor.handle(message).await,
            None => unreachable!(),
        };
    }

    async fn add<A: 'static +  Actor>(&mut self, id: A::Id) {
        let type_id = TypeId::of::<A>();

        let actor = A::activate(id).await;

        self.actor_managers.insert(type_id, Box::new(actor));
    }
}

pub fn start() {
    block_on(async {
        let mut sys = System {
            actor_managers: HashMap::new(),
        };

        let message = TestMessage {field: "hola".to_string()};
        sys.send::<TestActor, TestMessage>(43, message).await;

        let message = TestMessage {field: "adios".to_string()};
        sys.send::<TestActor, TestMessage>(43, message).await;
    });
}
