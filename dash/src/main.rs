use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, App, Arg};

use rocket_dyn_templates::Template;

use printnanny_dash::auth;
use printnanny_dash::debug;
use printnanny_dash::home;

#[tokio::main]
async fn main() -> Result<()> {
    let app_name = "printnanny-dash";
    let app = App::new(app_name)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!());
    rocket::build()
        .mount("/", home::routes())
        .mount("/debug", debug::routes())
        .mount("/login", auth::routes())
        .attach(Template::fairing())
        .launch()
        .await?;
    Ok(())
}
