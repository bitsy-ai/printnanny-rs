#[macro_use]
extern crate clap;

use anyhow::Result;
use clap::{Arg, Command};
use env_logger::Builder;
use futures_util::StreamExt;
use git_version::git_version;
use log::{error, info, warn, LevelFilter};

use printnanny_nats_client::client::wait_for_nats_client;
use printnanny_octoprint_models;
use printnanny_settings::sys_info;

const GIT_VERSION: &str = git_version!();
const DEFAULT_NATS_URI: &str = "nats://localhost:4223";

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny-nats-octoprint";

    let hostname = sys_info::hostname()?;
    let default_subject = format!("pi.{hostname}.octoprint.events.>");

    let app = Command::new(app_name)
        .author(crate_authors!())
        .about(crate_description!())
        .version(GIT_VERSION)
        .arg(
            Arg::new("nats_server_uri")
                .long("nats-server-uri")
                .takes_value(true)
                .default_value(DEFAULT_NATS_URI),
        )
        .arg(
            Arg::new("subject")
                .long("subject")
                .takes_value(true)
                .default_value(&default_subject),
        )
        .arg(
            Arg::new("workers")
                .long("workers")
                .takes_value(true)
                .default_value("4"),
        );

    let args = app.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = args.occurrences_of("v");
    match verbosity {
        0 => {
            builder.filter_level(LevelFilter::Warn).init();
        }
        1 => {
            builder.filter_level(LevelFilter::Info).init();
        }
        2 => {
            builder.filter_level(LevelFilter::Debug).init();
        }
        _ => builder.filter_level(LevelFilter::Trace).init(),
    };

    let nats_server_uri = args.get_one::<String>("nats_server_uri").unwrap();
    let subject = args.get_one::<String>("subject").unwrap();
    let workers = args.get_one::<usize>("workers").map(|v| v.to_owned());

    info!(
        "Attempting to initialize NATS connection to {}",
        &nats_server_uri
    );

    let nats_client = wait_for_nats_client(&nats_server_uri, &None, false, 2000).await?;

    let subscriber = nats_client.subscribe(subject.clone()).await.unwrap();
    warn!(
        "Listening on {} where subject={}",
        &nats_server_uri, &subject
    );

    subscriber
        .for_each_concurrent(workers, |message| async {})
        .await;

    Ok(())
}
