use acteur::{System, Actor, Handle, Secretary};
use async_trait::async_trait;

fn main() {
    start();
}

pub fn start() {
    let sys = System::new();

    for i in 0..1u32 {
        let message = TestMessage {
            field: i,
        };

        sys.send::<TestActor, TestMessage>(43.to_string(), message);
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    //println!("Waiting...");

    sys.block();

    //println!("All messages sent!");
}

#[derive(Debug)]
struct TestMessage {
    pub field: u32,
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
        //println!("Actor {} reporting sir!", id);
        TestActor {
            id: id.to_string(),
            values: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl Handle<TestMessage> for TestActor {
    async fn handle(&mut self, message: TestMessage, secretary: Secretary) {
        //println!("I'm actor {} and I'm sending a message for actor {}", self.id, message.field);
        
        if message.field > 100 {
            panic!();
        }

        secretary.send::<TestActor, TestMessage>((message.field + 1).to_string(), TestMessage {
            field: message.field + 1,
        }).await;
    }
}
