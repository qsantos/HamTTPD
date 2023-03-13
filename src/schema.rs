// @generated automatically by Diesel CLI.

diesel::table! {
    message (id) {
        id -> Int4,
        created -> Timestamp,
        author -> Text,
        content -> Text,
    }
}
