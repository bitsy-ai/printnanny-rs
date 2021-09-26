use anyhow::{ Result };
// use log::{ info };
use simple_logger::SimpleLogger;

use clap::{ Arg, App, SubCommand };
// use clap::{ AppSettings };
use clap::{
    crate_authors,
    crate_description,
    crate_name,
    crate_version
};
use printnanny::installer::{
    update_ansible
};


// #[tokio::main]
fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let app = App::new(crate_name!())
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
        .subcommand(SubCommand::with_name("update")
            .about("Update Print Nanny components")
            .subcommand(SubCommand::with_name("ansible")
            .arg(Arg::with_name("ansible_version")
                .long("ansible-version")
                .help("Ansible version to install"))
            )
        );

    let app_m = app.get_matches();
    match app_m.subcommand() {
        ("update", Some(sub_m))  => {
            match sub_m.subcommand(){
                ("ansible", _cmd_m) => {
                    update_ansible(app_m.value_of("ansible_version"))?;
                },
                _ => {

                },
            }
        }, // update
        _ => {}, // no subcommand specified
    }
    Ok(())
}
