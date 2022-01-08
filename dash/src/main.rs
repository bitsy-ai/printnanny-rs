#[macro_use] extern crate rocket;
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

use rocket::State;
use rocket::http::{Status, ContentType};
use rocket::form::{
    Form,
    Contextual,
    FromForm,
    FromFormField,
    Context,
};
use rocket::http::RawStr;
use rocket::fs::{FileServer, TempFile, relative};
use rocket_auth::{ Users, User };
use rocket_dyn_templates::Template;
use sqlx::sqlite::{ SqlitePool, SqliteConnectOptions};
use sqlx::prelude::ConnectOptions;

use services::printnanny_api::ApiService;

struct Config {
    base_url: String,
    path: String
}

#[derive(Debug, FromForm)]
struct Password<'v> {
    #[field(validate = len(6..))]
    #[field(validate = eq(self.second))]
    first: &'v str,
    #[field(validate = eq(self.first))]
    second: &'v str,
}

#[derive(Debug, FromFormField)]
enum Rights {
    Public,
    Reserved,
    Exclusive,
}

#[derive(Debug, FromFormField)]
enum Category {
    Biology,
    Chemistry,
    Physics,
    #[field(value = "CS")]
    ComputerScience,
}

#[derive(Debug, FromForm)]
struct Account<'v> {
    #[field(validate = contains('@').or_else(msg!("invalid email address")))]
    email: &'v str,
    #[field(validate = eq(true).or_else(msg!("Please agree to submit anonymous debug logs")))]
    analytics: bool,
}


#[get("/")]
fn index(option: Option<User>) -> Template {
    let mut context = HashMap::new();
    if let Some(user) = option {
        context.insert("user", user);
        Template::render("index", context)
    } else {
        Template::render("authemail", &Context::default())
    }
}

// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.
#[post("/login", data = "<form>")]
async fn submit<'r>(form: Form<Contextual<'r, Account<'r>>>, config: &State<Config>) -> (Status, Template) {
    info!("Received form response {:?}", form);
    let template = match form.value {
        Some(ref signup) => {
            let service = ApiService::new(&config.path, &config.base_url).await;
            match service {
                Ok(s) => {
                    let res = s.auth_email_create(signup.email.to_string()).await;
                    match res {
                        Ok(_) => Template::render("authcode", &form.context),
                        Err(e) => {
                            error!("{}", e);
                            Template::render("error", &form.context)
                        }
                    }
                },
                Err(e) => {
                    error!("{}", e);
                    Template::render("error", &form.context)
                }
            }
        }
        None => Template::render("authemail", &form.context),
    };

    (form.context.status(), template)
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
        .default_value("https://print-nanny.com"));
    
    let app_m = app.get_matches();
    let db = app_m.value_of("db").unwrap();
    let config = app_m.value_of("config").unwrap();
    let base_url = app_m.value_of("base_url").unwrap();

    SqliteConnectOptions::from_str(&db)?
        .create_if_missing(true)
        .connect().await?;
    let conn = SqlitePool::connect(&db).await?;
    let users: Users = conn.into();
    users.create_table().await?;

    let config = Config{ path: config.to_string(), base_url: base_url.to_string()};

    rocket::build()
        .mount("/", routes![index, submit])
        .attach(Template::fairing())
        .mount("/", FileServer::from(relative!("/static")))
        .manage(users)
        .manage(config)
        .launch().await?;
    Ok(())
}
