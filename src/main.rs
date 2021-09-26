use std::path::{ Path, PathBuf };
use anyhow::{ Result };
// use log::{ info };

use clap::{ Arg, App, SubCommand, AppSettings };
// use clap::{ AppSettings };
use clap::{
    crate_authors,
    crate_description,
    crate_name,
    crate_version
};
use printnanny::installer::python::{
    pip_install
};
use printnanny::installer::ansible::{
    ansible_pull
};

fn update_ansible_core(
    args: &clap::ArgMatches,
    venv_dir: &PathBuf
) -> Result<std::process::Output> {
    let ansible_venv = venv_dir.join("ansible/");
    pip_install(
        "ansible",
        args.value_of("ansible_version").unwrap(),
        ansible_venv
    )
}


fn update_all(
    args: &clap::ArgMatches,
    install_dir: &PathBuf,
    venv_dir: &PathBuf,
    log_dir: &PathBuf
) -> Result<()> {
    update_ansible_core(args, venv_dir)?;
    ansible_pull(
        args.value_of("ansible_pull_scm").unwrap(),
        args.value_of("ansible_pull_checkout").unwrap(),
        install_dir, 
        venv_dir, 
        log_dir
    )?;
    Ok(())
}


// #[tokio::main]
fn main() -> Result<()> {

    let home_dir = dirs::home_dir().unwrap();
    let crate_name = crate_name!();

    simple_logger::init_with_level(log::Level::Info).unwrap();
    let app = App::new(crate_name)
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
        .arg(Arg::with_name("install_dir")
            .long("Install Print Nanny to target directory")
            .default_value(home_dir.to_str().unwrap())
        )
        .subcommand(
            SubCommand::with_name("update")
            // tracking ansible version
            .arg(Arg::with_name("ansible_version")
            .long("ansible-version")
            .help("Ansible version to install")
            .default_value("==4.6.0"))
            // tracking ansible role version
            .arg(Arg::with_name("ansible_pull_scm")
                .long("ansible-pull-scm")
                .help("Pull playbooks/roles from remote git repository")
                .value_name("GIT REPO")
                .default_value("git@github.com:bitsy-ai/ansible-role-print-nanny.git"))
            .arg(Arg::with_name("ansible_pull_checkout")
                .long("ansible-pull-checkout")
                .value_name("COMMITISH")
                .default_value("main"))

            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about("Update Print Nanny components")
            .subcommand(SubCommand::with_name("ansible-core"))
            .subcommand(SubCommand::with_name("ansible-roles"))
            .subcommand(SubCommand::with_name("all"))

        );

    let app_m = app.get_matches();
    let dotdir = format!(".{}", crate_name);
    let install_dir = Path::new(app_m.value_of("install_dir").unwrap()).join(dotdir);
    let log_dir = install_dir.join("logs/");
    let venv_dir = install_dir.join("venv/");
    match app_m.subcommand() {
        ("update", Some(sub_m))  => {
            match sub_m.subcommand(){
                ("ansible-core", Some(cmd_m)) => {
                    update_ansible_core(cmd_m, &venv_dir)?;
                }, // update ansible
                ("all", Some(cmd_m)) => {
                    update_all(sub_m, &install_dir, &venv_dir, &log_dir)?;
                }, // no target specified, update all
                _ => {}
            }
        }, // update
        _ => {
        }, 
    }
    Ok(())
}
