use anyhow::Result;
use printnanny_nats_apps::event::NatsEvent;
use printnanny_nats_apps::request_reply::{NatsReply, NatsRequest};
use printnanny_nats_client::subscriber::NatsSubscriber;

use env_logger::Builder;
use log::LevelFilter;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let app = NatsSubscriber::<NatsEvent, NatsRequest, NatsReply>::clap_command(None);
    let args = app.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let mut builder = Builder::new();
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

    let worker = NatsSubscriber::<NatsEvent, NatsRequest, NatsReply>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}
