
use anyhow::{ Result };
use std::io::{self, Write};
use log::{ info, warn };
use std::process::{ Command, Output};

pub fn install_ansible(args: [&str; 2]) -> Result<Output> {
    let output = Command::new("pip3")
        .args(args)
        .output()
        .expect("Ansible install failed");
    println!("status: {}", output.status);
    info!("{}", String::from_utf8_lossy(&output.stdout));
    warn!("{}", String::from_utf8_lossy(&output.stderr));
    Ok(output)
}

pub fn update_ansible(version: Option<&str>) -> Result<()> {
    let output = match version {
        Some(version) => {
            let v = format!("ansible=={}", version.to_string());
            install_ansible(["install", &v]);
        },
        _ => {
            install_ansible(["install", "ansible"]);
        }
    };
    Ok(())
}