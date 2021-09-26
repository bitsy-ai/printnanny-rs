

use anyhow::{ Result };
use std::path::{ Path, PathBuf };
use std::process::{ Command, Output};
use log::{ info, warn };

pub fn ansible_pull(
    scm: &str,
    commit: &str,
    install_dir: &PathBuf,
    venv_dir: &PathBuf,
    log_dir: &PathBuf,
) -> Result<Output> {
    let dir = install_dir.join("ansible-role-print-nanny");
    let ansiblepull = venv_dir.join("ansible/bin/ansible-pull");
    // let logfile = log_dir.join("ansible-role-print-nanny.log");
    let output = Command::new(ansiblepull)
        .args([
            "-C",
            commit,
            "-d",
            dir.to_str().unwrap(),
            "-i",
            "localhost, tasks/main.yml",
            "-U",
            scm
        ])
        .output()
        .expect("ansible-pull failed");
    info!("{}", String::from_utf8_lossy(&output.stdout));

    if !output.stderr.is_empty() {
        warn!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output)
}