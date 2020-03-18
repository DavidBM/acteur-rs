use acteur::{Actor, Assistant, Handle, System};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::time::SystemTime;

fn main() {
    start();
}

pub fn start() {
    let start = SystemTime::now();
    println!("Time to start: {:?}", start);

    let sys = System::new();

    for i in 0..1u32 {
        let message = TestMessage { field: i };

        sys.send_sync::<TestActor<u32>, TestMessage>(43, message);
    }

    sys.wait_until_stopped();

    let end = SystemTime::now();

    println!("Duration until shutdown: {:?}", end.duration_since(start));
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

/// The only missing piece for shortening this would be: https://github.com/rust-lang/rust/issues/13231
#[async_trait]
impl<T: 'static + Hash + Clone + Eq + Sync + Send + Debug> Handle<TestMessage> for TestActor<T> {
    async fn handle(&mut self, message: TestMessage, assistant: Assistant<TestActor<T>>) {
        /*println!(
            "I'm actor {:?} and I'm sending a message for actor {:?}",
            self.id, message.field + 1
        );*/

        if message.field > 1_000_000 {
            println!("Time of end: {:?}", SystemTime::now());
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
