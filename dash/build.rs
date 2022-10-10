use static_files::NpmBuild;

fn main() -> std::io::Result<()> {
    NpmBuild::new("./ui")
        .executable("npm")
        .install()?
        .run("build")?
        .target("./ui/dist")
        .to_resource_dir()
        .build()
}
