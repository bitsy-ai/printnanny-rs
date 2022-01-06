#[macro_use] extern crate rocket;
use std::str::FromStr;

use anyhow::{ Result };
use clap::{ 
    Arg,
    App,
    crate_version,
    crate_authors,
    crate_description
};

use rocket::http::{Status, ContentType};
use rocket::form::{Form, Contextual, FromForm, FromFormField, Context};
use rocket::fs::{FileServer, TempFile, relative};
use rocket_auth::{ Users, User };
use rocket_sync_db_pools::Config;
use rocket_dyn_templates::Template;
use sqlx::sqlite::{ SqlitePool, SqliteConnectOptions};
use sqlx::prelude::ConnectOptions;
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
struct Submission<'v> {
    #[field(validate = len(1..))]
    title: &'v str,
    #[field(validate = len(1..=250))]
    r#abstract: &'v str,
    #[field(validate = ext(ContentType::PDF))]
    file: TempFile<'v>,
    #[field(validate = len(1..))]
    category: Vec<Category>,
    rights: Rights,
    ready: bool,
}

#[derive(Debug, FromForm)]
struct Account<'v> {
    #[field(validate = contains('@').or_else(msg!("invalid email address")))]
    email: &'v str,
    analytics: bool,

}

#[derive(Debug, FromForm)]
struct SubmitStep1<'v> {
    account: Account<'v>,
    // submission: Submission<'v>,
}

#[get("/")]
fn index(option: Option<User>) -> Template {
    if let Some(user) = option {
        Template::render("index", &Context::default())
    } else {
        Template::render("login", &Context::default())
    }
}

// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.
#[post("/login", data = "<form>")]
fn submit<'r>(form: Form<Contextual<'r, SubmitStep1<'r>>>) -> (Status, Template) {
    let template = match form.value {
        Some(ref submission) => {
            println!("submission: {:#?}", submission);
            Template::render("success", &form.context)
        }
        None => Template::render("index", &form.context),
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
        .arg(Arg::with_name("db")
        .help("Path to sqlite.db")
        .default_value("sqlite://data.db")
        .takes_value(true));
    
    let app_m = app.get_matches();
    let db = app_m.value_of("db").unwrap();

    let options = SqliteConnectOptions::from_str(&db)?
        .create_if_missing(true)
        .connect().await?;
    let conn = SqlitePool::connect(&db).await?;
    let users: Users = conn.into();
    users.create_table().await?;

    rocket::build()
        .mount("/", routes![index, submit])
        .attach(Template::fairing())
        .mount("/", FileServer::from(relative!("/static")))
        .manage(users)
        .launch();

    // let rocket::build()
    //     .mount("/", routes![index, submit])
    //     .attach(Template::fairing())
    //     .mount("/", FileServer::from(relative!("/static")))
    //     // .manage(users)
    //     .launch();
    Ok(())
}
