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
    let dir = settings.paths.snapshot_dir.clone();
    let dir_entry = fs::read_dir(&dir).map_err(|e| NotFound(e.to_string()))?;

    match dir_entry.last() {
        Some(last) => {
            let last = last.map_err(|e| NotFound(e.to_string()))?;
            let result = NamedFile::open(last.path())
                .await
                .map_err(|e| NotFound(e.to_string()))?;
            Ok(result)
        }
        None => Err(NotFound(format!(
            "Failed to read directory {}",
            dir.display().to_string()
        ))),
    }
}

#[launch]
fn rocket() -> _ {
    let settings = PrintNannySettings::new().expect("Failed to initialize PrintNannySettings");

    rocket::build().manage(settings).mount("/", routes![jpeg])
}
