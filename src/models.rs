use rocket::serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Todo {
    pub id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateTodo<'r> {
    pub content: &'r str,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(crate = "rocket::serde")]
pub struct UpdateTodo<'r> {
    pub content: &'r str,
}
