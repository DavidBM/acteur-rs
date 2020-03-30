#[async_trait]
pub(crate) trait ServiceEnvelope: Send + Debug {
    type Actor: Service;

    async fn dispatch(&mut self, actor: &mut Self::Service);
}
