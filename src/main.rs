use acteur::{System, Actor, Handle};
use async_std::task::block_on;
use async_trait::async_trait;

fn main() {
    start();
}

pub fn start() {
    block_on(async {
        let sys = System::new();

        for i in 0..10_000_000 {
            let message = TestMessage {
                field: i.to_string(),
            };

            sys.send::<TestActor, TestMessage>(43.to_string(), message)
                .await;
        }

        println!("All messages sent!");
    });
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
