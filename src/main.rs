use std::collections::HashMap;
use std::env;
use std::ops::DerefMut;
use std::process::Command;
use std::sync::{Arc, Mutex};

use askama::Template;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get_service;
use axum::{
    extract::{Query, State},
    response::Html,
    routing::{get, post},
    Form, Router, Server,
};
use diesel::{Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use serde::Deserialize;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

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

async fn root(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    form: Option<Form<MessageForm>>,
) -> Html<String> {
    let user = params
        .get("dn")
        .and_then(|distinguished_name| User::from_dn(distinguished_name));

    let mut guard = state
        .db
        .lock()
        .expect("Failed to acquire lock on database connection");
    let db = guard.deref_mut();

    if let Some(user) = &user {
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
    }

    let messages = self::schema::message::dsl::message
        .order(self::schema::message::columns::created.desc())
        .load::<models::Message>(db)
        .expect("Error loading messages");

    #[derive(Template)]
    #[template(path = "index.html")]
    struct IndexTemplate {
        user: Option<User>,
        messages: Vec<models::Message>,
    }

    let template = IndexTemplate { user, messages };
    Html(template.render().unwrap())
}

#[derive(Deserialize)]
struct VisitorCertificateForm {
    nickname: String,
}

async fn visitor(form: Option<Form<VisitorCertificateForm>>) -> impl IntoResponse {
    let nickname = &form.unwrap().nickname;

    // create client key
    let status = Command::new("openssl")
        .args(["genrsa", "-out", "client.key", "1024"])
        .status()
        .expect("Failed to run genrsa");
    assert!(status.success(), "genrsa failed");

    // create client certificate request
    let status = Command::new("openssl")
        .args([
            "req",
            "-config",
            "openssl.conf",
            "-new",
            "-key",
            "client.key",
            "-out",
            "client.csr",
            "-subj",
            &format!("/{CALLSIGN_OID}=FAKE/{DISPLAYNAME_OID}={nickname} (Visitor)/{EMAIL_OID}=fake@example.com"),
        ])
        .status()
        .expect("Failed to run req");
    assert!(status.success(), "req failed");

    // create client certificate
    let status = Command::new("openssl")
        .args([
            "x509",
            "-req",
            "-CA",
            "local-ca.pem",
            "-CAkey",
            "local-ca.key",
            "-CAcreateserial",
            "-in",
            "client.csr",
            "-out",
            "client.pem",
        ])
        .status()
        .expect("Failed to run x509");
    assert!(status.success(), "x509 failed");

    // export private key and certificate to PKCS#12
    let status = Command::new("openssl")
        .args([
            "pkcs12",
            "-export",
            "-out",
            "client.p12",
            "-in",
            "client.pem",
            "-inkey",
            "client.key",
            "-passout",
            "pass:",
        ])
        .status()
        .expect("Failed to run pkcs12");
    assert!(status.success(), "pkcs12 failed");

    let cert = std::fs::read("client.p12").expect("Could not read client.p12");
    (
        [
            (header::CONTENT_TYPE, "application/x-pkcs12"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"client.p12\"",
            ),
        ],
        cert,
    )
}

async fn about(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    let user = params
        .get("dn")
        .and_then(|distinguished_name| User::from_dn(distinguished_name));

    #[derive(Template)]
    #[template(path = "about.html")]
    struct AboutTemplate {
        user: Option<User>,
    }

    let template = AboutTemplate { user };
    Html(template.render().unwrap())
}

async fn error_404() -> Html<String> {
    #[derive(Template)]
    #[template(path = "404.html")]
    struct Error404Template {}

    let template = Error404Template {};
    Html(template.render().unwrap())
}

fn has_ca() -> bool {
    // extract modulus of secret key
    let sk_mod = Command::new("openssl")
        .args(["rsa", "-noout", "-modulus", "-in", "local-ca.key"])
        .output()
        .expect("Failed to check CA's secret key's modulus");
    if !sk_mod.status.success() {
        return false;
    }
    let sk_mod = sk_mod.stdout;

    // extract modulus of public key
    let pk_mod = Command::new("openssl")
        .args(["x509", "-noout", "-modulus", "-in", "ca.pem"])
        .output()
        .expect("Failed to check CA's public key's modulus");
    if !pk_mod.status.success() {
        return false;
    }
    let pk_mod = pk_mod.stdout;

    sk_mod == pk_mod
}

fn ensure_ca() {
    if has_ca() {
        println!("Local CA certificate present, using it");
        return;
    }
    println!("No local CA certificate, generating one");

    // generate secret key
    let status = Command::new("openssl")
        .args(["genrsa", "-out", "local-ca.key", "1024"])
        .status()
        .expect("Failed to generate secret key for local CA");
    assert!(
        status.success(),
        "Generating secret key for local CA failed"
    );

    // create certificate
    let status = Command::new("openssl")
        .args([
            "req",
            "-new",
            "-x509",
            "-key",
            "local-ca.key",
            "-out",
            "local-ca.pem",
            "-subj",
            "/CN=127.0.0.1",
        ])
        .status()
        .expect("Failed to create certificate for local CA");
    assert!(status.success(), "Creating certificate for local CA failed");

    // combine LoTW and local CAs
    let mut lotw_ca = std::fs::read("LoTW-root.pem").expect("Failed to read LoTW CA");
    let local_ca = std::fs::read("local-ca.pem").expect("Failed to read local CA");
    let mut combined_ca = local_ca;
    combined_ca.append(&mut lotw_ca);
    std::fs::write("ca.pem", combined_ca).expect("Failed to create combined CA file");
}

#[tokio::main]
async fn main() {
    ensure_ca();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let database_url =
        env::var("DATABASE_URL").expect("Please set the DATABASE_URL environment variable");
    let db = PgConnection::establish(&database_url).expect("Failed to connect to database");

    let shared_state = Arc::new(AppState { db: Mutex::new(db) });

    let app = Router::new()
        .route("/", get(root).post(root))
        .route("/about.html", get(about))
        .route("/visitor", post(visitor))
        .nest_service("/static", get_service(ServeDir::new("./static")))
        .layer(TraceLayer::new_for_http())
        .fallback(error_404)
        .with_state(shared_state);
    println!("Listening to http://0.0.0.0:8000");
    Server::bind(
        &"0.0.0.0:8000"
            .parse()
            .expect("Could not parse bind address"),
    )
    .serve(app.into_make_service())
    .await
    .unwrap();
}
