use acteur::{Actor, Handle, Secretary, System};
use async_trait::async_trait;

fn main() {
    start();
}

pub fn start() {
    let sys = System::new();

    for i in 0..1u32 {
        let message = TestMessage { field: i };

        sys.send::<TestActor, TestMessage>(43.to_string(), message);
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    sys.block();
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
            id,
            values: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl Handle<TestMessage> for TestActor {
    async fn handle(&mut self, message: TestMessage, secretary: Secretary) {
        println!(
            "I'm actor {} and I'm sending a message for actor {}",
            self.id, message.field
        );

        if message.field > 1_000_000 {
            return secretary.stop_system().await;
        }

        secretary
            .send::<TestActor, TestMessage>(
                (message.field + 1).to_string(),
                TestMessage {
                    field: message.field + 1,
                },
            )
            .await;
    }
}
