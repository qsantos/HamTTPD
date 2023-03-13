use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};

#[derive(Queryable)]
pub struct Message {
    pub id: i32,
    pub created: NaiveDateTime,
    pub author: String,
    pub content: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::message)]
pub struct NewMessage<'a> {
    pub author: &'a str,
    pub content: &'a str,
}
