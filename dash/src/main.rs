use anyhow::{ Result};
use clap::{ 
    Arg,
    App,
    crate_version,
    crate_authors,
    crate_description
};

use rocket_dyn_templates::Template;

use printnanny_dash::config::{ Config };
use printnanny_dash::home;
use printnanny_dash::auth;


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
    let config = app_m.value_of("config").unwrap();
    let base_url = app_m.value_of("base_url").unwrap();

    // SqliteConnectOptions::from_str(&db)?
    //     .create_if_missing(true)
    //     .connect().await?;
    // let conn = SqlitePool::connect(&db).await?;
    // users.create_table().await?;

    let config = Config{ path: config.to_string(), base_url: base_url.to_string()};

    rocket::build()
        .mount("/", home::routes())
        .mount("/login", auth::routes())
        .attach(Template::fairing())
        .manage(config)
        .launch().await?;
    Ok(())
}
