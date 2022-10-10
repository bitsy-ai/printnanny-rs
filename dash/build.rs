use static_files::NpmBuild;

fn main() -> std::io::Result<()> {
    NpmBuild::new("./ui")
        .executable("npm")
        .install()
        .expect("Failed to run npm install")
        .run("build")
        .expect("Failed to run npm build")
        .target("./ui/dist")
        .to_resource_dir()
        .build()
}
