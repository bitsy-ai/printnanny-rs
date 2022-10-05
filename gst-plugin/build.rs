extern crate flatc_rust; // or just `use flatc_rust;` with Rust 2018 edition.

use std::path::Path;

fn main() {
    gst_plugin_version_helper::info();

    println!("cargo:rerun-if-changed=src/nnstreamer.fbs");
    flatc_rust::run(flatc_rust::Args {
        inputs: &[Path::new("src/nnstreamer.fbs")],
        out_dir: Path::new("target/flatbuffers/"),
        ..Default::default()
    })
    .expect("flatc");

    println!("cargo:rustc-link-lib=nnstreamer");
}
