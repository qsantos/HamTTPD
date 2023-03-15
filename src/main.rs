use std::collections::HashMap;
use std::env;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
    routing::get,
    Form, Router, Server,
};
use diesel::{Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use serde::Deserialize;

pub mod models;
pub mod schema;

const CALLSIGN_OID: &str = "1.3.6.1.4.1.12348.1.1";
const DISPLAYNAME_OID: &str = "CN";
const EMAIL_OID: &str = "emailAddress";

struct AppState {
    db: Mutex<PgConnection>,
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
struct IndexTemplate {
    user: User,
    messages: Vec<models::Message>,
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

    let mut guard = state
        .db
        .lock()
        .expect("Failed to acquire lock on database connection");
    let db = guard.deref_mut();

    if let Some(form) = form {
        let message = models::NewMessage {
            author: &user.callsign,
            content: &form.message,
        };
        diesel::insert_into(schema::message::table)
            .values(&message)
            .execute(db)
            .expect("Could not insert message");
    }

    let messages = self::schema::message::dsl::message
        .order(self::schema::message::columns::created.desc())
        .load::<models::Message>(db)
        .expect("Error loading messages");

    let template = IndexTemplate { user, messages };
    Html(template.render().unwrap())
}

#[tokio::main]
async fn main() {
    let database_url =
        env::var("DATABASE_URL").expect("Please set the DATABASE_URL environment variable");
    let db = PgConnection::establish(&database_url).expect("Failed to connect to database");

    let shared_state = Arc::new(AppState { db: Mutex::new(db) });

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
