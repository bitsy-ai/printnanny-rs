use static_files::NpmBuild;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let base_dir = env!("CARGO_MANIFEST_DIR");
    let ui_dir = PathBuf::from(base_dir).join("ui");
    let dist_dir = ui_dir.join("dist");
    NpmBuild::new(ui_dir)
        .executable("npm")
        .install()?
        .run("build")?
        .target(dist_dir)
        .to_resource_dir()
        .build()
}
