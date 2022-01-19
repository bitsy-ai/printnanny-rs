use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, App, Arg};

use rocket_dyn_templates::Template;

use printnanny_dash::auth;
use printnanny_dash::home;
use printnanny_services::config::{ApiConfig, PrintNannyConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let app_name = "printnanny-dash";
    let app = App::new(app_name)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .takes_value(true)
                .help("Path to Config.toml (see env/ for examples)"),
        );
    let app_m = app.get_matches();
    let config = app_m.value_of("config");

    let config = PrintNannyConfig::new(config)?;

    rocket::build()
        .mount("/", home::routes())
        .mount("/login", auth::routes())
        .attach(Template::fairing())
        .manage(config)
        .launch()
        .await?;
    Ok(())
}
