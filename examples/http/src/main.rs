use acteur::{Acteur, Actor, ActorAssistant, Respond};
use tide::{Request, Response};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct User {
    id: u64,
    username: String,
}

#[async_trait::async_trait]
impl Actor for User {
    type Id = u64;

    async fn activate(id: Self::Id, _: &ActorAssistant<Self>) -> Self {
        User {
            id,
            // Here you can check the DB to load the actor
            username: "Test".to_string(),
        }
    }
}

#[derive(Debug)]
struct UpdateUserUsername {
    username: String,
}

#[async_trait::async_trait]
impl Respond<UpdateUserUsername> for User {
    type Response = User;

    // If your struct can be cloned, you can respond with yourself.
    async fn handle(&mut self, message: UpdateUserUsername, _: &ActorAssistant<Self>) -> User {
        self.username = message.username;
        self.clone()
    }
}

#[derive(Debug)]
struct GetUser;

#[async_trait::async_trait]
impl Respond<GetUser> for User {
    type Response = User;

    async fn handle(&mut self, _: GetUser, _: &ActorAssistant<Self>) -> User {
        self.clone()
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    let acteur = Acteur::new();
    let mut app = tide::with_state(acteur);

    app.at("user/:user_id")
        .get(|req: Request<Acteur>| async move {
            let user_id = req.param::<u64>("user_id")?;

            match req.state().call_actor::<User, _>(user_id, GetUser).await {
                Ok(response) => Ok(Response::new(tide::http::StatusCode::Ok).body_json(&response)?),
                Err(_message) => Ok(Response::new(tide::http::StatusCode::InternalServerError)),
            }
        });

    app.at("user/:user_id/username/:username")
        .put(|req: Request<Acteur>| async move {
            let user_id = req.param::<u64>("user_id")?;
            let username = req.param::<String>("username")?;

            match req
                .state()
                .call_actor::<User, _>(user_id, UpdateUserUsername { username })
                .await
            {
                Ok(response) => Ok(Response::new(tide::http::StatusCode::Ok).body_json(&response)?),
                Err(_message) => Ok(Response::new(tide::http::StatusCode::InternalServerError)),
            }
        });

    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
