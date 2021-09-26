
use anyhow::{ Result };
use log::{ info, warn };
use std::process::{ Command, Output};

pub fn ensure_build_deps(python: &str) -> Result<Output> {
    let output = Command::new(python)
        .args([
            "-m",
            "pip",
            "install",
            "--upgrade",
            "pip",
            "setuptools",
            "wheel"
        ])
        .output()
        .expect("Failed to upgrade pip, setuptools, wheel in virtual environment. Run `printnanny clean ansible` and try again.");
    info!("{}", String::from_utf8_lossy(&output.stdout));

    if !output.stderr.is_empty() {
        warn!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output)
}

pub fn create_venv(target: &str) -> Result<Output> {
    let args = [
        "-m",
        "venv",
        target
    ];
    let output= Command::new("python3")
        .args(args)
        .output()
        .expect(&format!("Failed to initialize venv {}", target));
    
    info!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        warn!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output)
}


pub fn pip_install(pkg: &str, semver: &str, venv: std::path::PathBuf) -> Result<Output> {
    let version_str = format!("{}{}", &pkg, &semver);
    let venv_str = venv.to_str().unwrap();
    let pip = venv.join("bin/pip");
    let python = venv.join("bin/python");

    if venv.exists() {
        info!("Using existing virtual environment {}", venv_str)
    } else {
        create_venv(venv_str)?;
        info!("Created new virtual environment {}", venv_str)
    }
    ensure_build_deps(python.to_str().unwrap())?;
    let args = vec![
        "install",
        "--upgrade",
        &version_str
    ];
    let output = Command::new(pip.to_str().unwrap())
        .args(args)
        .output()
        .expect("Pip install failed");
    
    info!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        warn!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output)
}