use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

async fn root(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    form: Option<Form<MessageForm>>,
) -> Html<String> {
    let distinguished_name = params
        .get("dn")
        .expect("Missing dn parameter in query string");
    let parts: HashMap<_, _> = distinguished_name
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| part.split_once('=').expect("Unexpected dn format"))
        .collect();

    let callsign = parts.get(CALLSIGN_OID).expect("Missing callsign in dn");
    let display_name = parts
        .get(DISPLAYNAME_OID)
        .expect("Missing display name in dn");
    let email = parts.get(EMAIL_OID).expect("Missing email in dn");

    if let Some(form) = form {
        state.messages.lock().unwrap().push(Message {
            created: Utc::now(),
            author: callsign.to_string(),
            contents: form.message.to_string(),
        });
    }

    let welcome = format!("Hello <a href=\"mailto:{email}\">{display_name}</a>. Your call sign is <strong>{callsign}</strong>");

    let messages = state.messages.lock().unwrap();
    let messages: Vec<&str> = messages
        .iter()
        .map(|message| message.contents.as_str())
        .collect();

    Html(welcome + "<br><form method=\"post\" action=\"\"><input type=\"text\" name=\"message\" /></form>" + &messages.join(" "))
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
