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
            error_state = oclock::client::handler::send_command(client_args.command)
        },
        #[cfg(feature = "server")]
        OClockCommand::Server => oclock::server::handler::server(),
    };

    std::process::exit(if error_state {
        1
    } else {
        0
    });
}