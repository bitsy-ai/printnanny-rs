#[macro_use] extern crate rocket;

use std::time::Duration;
use std::str::FromStr;
use std::collections::HashMap;

use log::{ info };
use anyhow::{ Result};
use clap::{ 
    Arg,
    App,
    crate_version,
    crate_authors,
    crate_description
};

use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::fs::{FileServer, relative};
use rocket_dyn_templates::Template;


use printnanny_dash::config::{ Config };
use printnanny_dash::response::{ Reponse };
use printnanny_dash::auth;


#[get("/")]
fn index(jar: &CookieJar<'_>) -> Response {
    let token = jar.get_private("token");
    // let mut context = HashMap::new();
    match token {
        Some(_) => Response::Template(Template::render("index", {})),
        None => Response::Redirect(Redirect::to("/login"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let app_name = "printnanny-dash";
    let app = App::new(app_name)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::new("config")
        .long("config")
        .takes_value(true)
        .help("Path to Print Nanny installation")
        .default_value("/opt/printnanny"))
        .arg(Arg::new("db")
        .help("Path to sqlite.db")
        .default_value("sqlite://data.db")
        .takes_value(true))
        .arg(Arg::new("base_url")
        .long("base-url")
        .takes_value(true)
        .help("Base Print Nanny url")
        .default_value("https://print-nanny.com"))
        .arg(Arg::new("api_token")
        .long("api-token")
        .takes_value(true)
        .help("Base PrintNanny api token"));
    
    let app_m = app.get_matches();
    let db = app_m.value_of("db").unwrap();
    let config = app_m.value_of("config").unwrap();
    let base_url = app_m.value_of("base_url").unwrap();

    // SqliteConnectOptions::from_str(&db)?
    //     .create_if_missing(true)
    //     .connect().await?;
    // let conn = SqlitePool::connect(&db).await?;
    // users.create_table().await?;

    let config = Config{ path: config.to_string(), base_url: base_url.to_string()};

    rocket::build()
        .mount("/", routes![
            index,
        ])
        .mount("/login", printnanny_dash::auth::routes())
        .attach(Template::fairing())
        .mount("/", FileServer::from(relative!("/static")))
        .manage(config)
        .launch().await?;
    Ok(())
}
