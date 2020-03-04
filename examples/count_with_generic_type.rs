use acteur::{Actor, Assistant, Handle, System};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

fn main() {
    start();
}

pub fn start() {
    let sys = System::new();

    for i in 0..1u32 {
        let message = TestMessage { field: i };

        sys.send::<TestActor<u32>, TestMessage>(43, message);
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    sys.wait_until_stopped();
}

#[derive(Debug)]
struct TestMessage {
    pub field: u32,
}

#[derive(Debug)]
struct TestActor<T> {
    id: T,
    values: HashMap<u64, String>,
}

#[async_trait]
impl<T: 'static + Send + Sync + Eq + Clone + Hash + Debug> Actor for TestActor<T> {
    type Id = T;

    async fn activate(id: Self::Id) -> Self {
        //
        TestActor {
            id,
            values: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl<T: 'static + Hash + Clone + Eq + Sync + Send + Debug> Handle<TestMessage> for TestActor<T> {
    async fn handle(&mut self, message: TestMessage, assistant: Assistant) {
        println!(
            "I'm actor {:?} and I'm sending a message for actor {:?}",
            self.id, message.field
        );

        if message.field > 1_000_000 {
            return assistant.stop_system().await;
        }

        assistant
            .send::<TestActor<u32>, TestMessage>(
                message.field + 1,
                TestMessage {
                    field: message.field + 1,
                },
            )
            .await;
    }
}
