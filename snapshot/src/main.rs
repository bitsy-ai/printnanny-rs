#[macro_use]
extern crate rocket;
use std::fs;

use rocket::fs::NamedFile;
use rocket::response::status::NotFound;
use rocket::State;

use printnanny_settings::printnanny::PrintNannySettings;

#[get("/jpeg")]
async fn jpeg(state: &State<PrintNannySettings>) -> Result<NamedFile, NotFound<String>> {
    let settings = state;
    let dir_entry = fs::read_dir(settings.paths.snapshot_dir.clone())
        .map_err(|e| NotFound(e.to_string()))?
        .last()
        .unwrap()
        .map_err(|e| NotFound(e.to_string()))?;

    NamedFile::open(&dir_entry.path())
        .await
        .map_err(|e| NotFound(e.to_string()))
}

#[launch]
fn rocket() -> _ {
    let settings = PrintNannySettings::new().expect("Failed to initialize PrintNannySettings");

    rocket::build().manage(settings).mount("/", routes![jpeg])
}
