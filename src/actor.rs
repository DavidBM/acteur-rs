use async_trait::async_trait;
use std::fmt::Debug;
use std::hash::Hash;

#[async_trait]
pub trait Actor: Debug + Send + 'static {
    type Id: Eq + Hash + Send + Clone + Debug;

    async fn activate(id: Self::Id) -> Self;

    async fn deactivate(&mut self) -> Result<(), ()> {
        Ok(())
    }
}
