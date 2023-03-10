use std::collections::HashMap;

use axum::{extract::Query, response::Html, routing::get, Router, Server};

const CALLSIGN_OID: &str = "1.3.6.1.4.1.12348.1.1";
const DISPLAYNAME_OID: &str = "CN";
const EMAIL_OID: &str = "emailAddress";

async fn root(Query(params): Query<HashMap<String, String>>) -> Html<String> {
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

    Html(
        format!("Hello <a href=\"mailto:{email}\">{display_name}</a>. Your call sign is <strong>{callsign}</strong>"),
    )
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(root));
    Server::bind(
        &"0.0.0.0:8000"
            .parse()
            .expect("Could not parse bind address"),
    )
    .serve(app.into_make_service())
    .await
    .unwrap();
}
