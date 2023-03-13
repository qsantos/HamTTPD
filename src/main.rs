use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
    routing::get,
    Form, Router, Server,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const CALLSIGN_OID: &str = "1.3.6.1.4.1.12348.1.1";
const DISPLAYNAME_OID: &str = "CN";
const EMAIL_OID: &str = "emailAddress";

#[allow(dead_code)] // the fields are read through a template
struct Message {
    author: String,
    created: DateTime<Utc>,
    contents: String,
}

struct AppState {
    messages: Mutex<Vec<Message>>,
}

#[derive(Deserialize)]
struct MessageForm {
    message: String,
}

struct User {
    callsign: String,
    display_name: String,
    email: String,
}

impl User {
    fn from_dn(distinguished_name: &str) -> Option<User> {
        let parts: HashMap<&str, &str> = distinguished_name
            .split('/')
            .filter(|part| !part.is_empty())
            .map(|part| part.split_once('='))
            .collect::<Option<_>>()?;

        Some(User {
            callsign: parts.get(CALLSIGN_OID)?.to_string(),
            display_name: parts.get(DISPLAYNAME_OID)?.to_string(),
            email: parts.get(EMAIL_OID)?.to_string(),
        })
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    user: User,
    messages: &'a Vec<Message>,
}

async fn root(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    form: Option<Form<MessageForm>>,
) -> Html<String> {
    let distinguished_name = params
        .get("dn")
        .expect("Missing dn parameter in query string");
    let user = User::from_dn(distinguished_name).expect("Could not authenticate user");

    if let Some(form) = form {
        state
            .messages
            .lock()
            .expect("Failed to acquire lock on messages")
            .push(Message {
                created: Utc::now(),
                author: user.callsign.to_string(),
                contents: form.message.to_string(),
            });
    }

    let messages_guard = state
        .messages
        .lock()
        .expect("Failed to acquire lock on messages");

    let messages = messages_guard.deref();

    let template = IndexTemplate { user, messages };
    Html(template.render().unwrap())
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(AppState {
        messages: Mutex::new(vec![]),
    });

    let app = Router::new()
        .route("/", get(root).post(root))
        .with_state(shared_state);
    Server::bind(
        &"0.0.0.0:8000"
            .parse()
            .expect("Could not parse bind address"),
    )
    .serve(app.into_make_service())
    .await
    .unwrap();
}
