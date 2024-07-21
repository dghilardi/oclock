use clap::Parser;

use crate::cli::args::{OClockArgs, OClockCommand};

mod cli;

fn main() {
    env_logger::init();
    let args: OClockArgs = OClockArgs::parse();

    let mut error_state = false;
    match args.subcommand {
        #[cfg(feature = "client")]
        OClockCommand::Client(client_args) => {
            let reply = oclock::client::handler::invoke_server::<
                oclock::dto::command::OClockClientCommand,
                serde_json::Value,
            >(client_args.command.into());
            match &reply {
                Ok(serde_json::Value::String(msg)) => println!("{msg}"),
                Ok(rep) => println!(
                    "{}",
                    serde_json::to_string(rep).expect("Error serializing reply")
                ),
                Err(err) => eprintln!("{err}"),
            }
            error_state = reply.is_ok();
        }
        #[cfg(feature = "server")]
        OClockCommand::Server => oclock::server::handler::server(),
    };

    std::process::exit(if error_state { 1 } else { 0 });
}
