#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;
extern crate openssl;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

use std::collections::HashMap;
use std::env;

use rocket::http::Status;
use rocket::request::Form;
use rocket::response::Redirect;
use rocket_contrib::templates::Template;

use url_parser::UrlParser;

use crate::shortener::{Shortener, UrlShortener};

mod shortener;
mod schema;
mod db;
mod repository;
mod url_parser;

#[derive(FromForm, Debug)]
struct UrlShortenRequest {
    url: String,
    hash: Option<String>,
}

#[get("/<id>")]
fn lookup(id: String, conn: db::Connection) -> Result<Redirect, Status> {
    match db::urls::find_one(&conn, &id) {
        None => Err(Status::NotFound),
        Some(res) => {
            match UrlParser::parse(res.url) {
                Ok(url) => Ok(Redirect::permanent(url)),
                Err(_) => Err(Status::BadRequest)
            }
        },
    }
}

#[get("/")]
fn console() -> Template {
    let context: HashMap<&str, &str> = HashMap::new();
    Template::render("console", &context)
}

#[post("/shorten", data = "<request>")]
fn shorten(request: Form<UrlShortenRequest>, conn: db::Connection) -> Template {
    let random_hash = UrlShortener::new().next_id();

    let hash = match &request.hash {
        Some(user_hash) if !user_hash.is_empty() => user_hash,
        _ => &random_hash,
    };

    match db::urls::insert(&conn, hash, &request.url) {
        None => {
            let mut context: HashMap<&str, &str> = HashMap::new();
            context.insert("error", "the requested hash already exists");
            Template::render("console", &context)
        }
        Some(res) => {
            let mut context: HashMap<&str, &str> = HashMap::new();
            context.insert("hash", &res.hash);
            Template::render("console", &context)
        }
    }
}

#[catch(404)]
fn not_found() -> String {
    let error: serde_json::Value = serde_json::json!({
        "error": "given key could not be found"
    });

    error.to_string()
}

fn main() {
    let console_enabled = env::var("CONSOLE_ENABLED").ok();

    let rocket = rocket::ignite()
        .register(catchers![not_found])
        .attach(db::Connection::fairing())
        .attach(Template::fairing())
        .mount("/", routes!(lookup));

    match console_enabled {
        Some(var) if var.eq("true") =>
            &rocket.mount("/console", routes!(console, shorten)).launch(),
        _ => &rocket.launch()
    };
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn test_cache_store_and_get() {}
}