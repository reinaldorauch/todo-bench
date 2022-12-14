#[macro_use]
extern crate rocket;

pub mod models;

use models::{CreateTodo, Todo, UpdateTodo};
use rocket::fairing::{self, AdHoc};
use rocket::futures::TryStreamExt;
use rocket::serde::uuid::Uuid;
use rocket::{http::Status, serde::json::Json, Build, Rocket};
use rocket_db_pools::sqlx;
use rocket_db_pools::{Connection, Database};
use rocket_dyn_templates::{context, Template};

#[derive(Database)]
#[database("postgres_todo_bench")]
struct DbTodo(sqlx::PgPool);

#[get("/")]
async fn hello(mut db: Connection<DbTodo>) -> Template {
    let todos_result = sqlx::query!(r#"SELECT id, content FROM todo"#)
        .fetch(&mut *db)
        .map_ok(|r| Todo {
            id: r.id.to_string(),
            content: r.content,
        })
        .try_collect::<Vec<Todo>>()
        .await
        .map_err(|e| e.to_string());

    match todos_result {
        Ok(todos) => Template::render("home", context! { todos }),
        Err(err) => Template::render("err", context! { err }),
    }
}

#[get("/")]
async fn list_todos(mut db: Connection<DbTodo>) -> Result<Json<Vec<Todo>>, String> {
    let todos = sqlx::query!(r#"SELECT id, content FROM todo"#)
        .fetch(&mut *db)
        .map_ok(|r| Todo {
            id: r.id.to_string(),
            content: r.content,
        })
        .try_collect::<Vec<Todo>>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(todos))
}

#[post("/", data = "<todo_data>")]
async fn create_todo(
    mut db: Connection<DbTodo>,
    todo_data: Json<CreateTodo<'_>>,
) -> Result<Json<Todo>, String> {
    let res = sqlx::query!(
        "INSERT INTO todo VALUES (DEFAULT, $1) RETURNING id",
        todo_data.content
    )
    .fetch_one(&mut *db)
    .await
    .map_err(|e| e.to_string())?;

    let todo = sqlx::query!("SELECT * FROM todo WHERE id = $1", res.id)
        .fetch_one(&mut *db)
        .await
        .map(|r| Todo {
            id: r.id.to_string(),
            content: r.content,
        })
        .map_err(|e| e.to_string())?;

    Ok(Json(todo))
}

#[put("/<id>", data = "<todo_data>")]
async fn update_todo(
    mut db: Connection<DbTodo>,
    id: Uuid,
    todo_data: Json<UpdateTodo<'_>>,
) -> Result<Status, String> {
    sqlx::query!(
        "UPDATE todo SET content = $1 WHERE id = $2",
        todo_data.content,
        sqlx::types::Uuid::from_u128_le(id.to_u128_le())
    )
    .execute(&mut *db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Status::Ok)
}

#[delete("/<id>")]
async fn delete_todo(mut db: Connection<DbTodo>, id: Uuid) -> Result<Status, Status> {
    sqlx::query!(
        "DELETE FROM todo WHERE id = $1",
        sqlx::types::Uuid::from_u128_le(id.to_u128_le())
    )
    .execute(&mut *db)
    .await
    .map_err(|e| {
        println!("Error when deleting todo: {}", e.to_string());
        Status::InternalServerError
    })?;

    Ok(Status::Ok)
}

#[delete("/")]
async fn clear_todo(mut db: Connection<DbTodo>) -> Result<Status, Status> {
    sqlx::query!("DELETE FROM todo")
        .execute(&mut *db)
        .await
        .map_err(|e| {
            println!("Error when deleting todo: {}", e.to_string());
            Status::InternalServerError
        })?;

    Ok(Status::Ok)
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match DbTodo::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("db/migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                error!("Failed to initialize SQLx database: {}", e);
                Err(rocket)
            }
        },
        None => Err(rocket),
    }
}

#[launch]
fn rocket() -> Rocket<Build> {
    rocket::build()
        .attach(Template::fairing())
        .attach(DbTodo::init())
        .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        .mount("/", routes![hello])
        .mount(
            "/todos",
            routes![
                list_todos,
                create_todo,
                update_todo,
                delete_todo,
                clear_todo
            ],
        )
}
