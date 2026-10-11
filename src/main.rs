//! random-fx: a thin line that lives along the top edge of the screen.
//!
//!     random-fx              run the daemon
//!     random-fx zap [#rgb]   send an event to the running daemon

mod color;
mod config;
mod ease;
mod edge;
mod ipc;
mod line;
mod machine;
mod motion;
mod palette;
mod pill;
mod rng;
mod signals;
mod states;
mod surface;

use std::process::ExitCode;
use std::sync::mpsc;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = if args.is_empty() {
        daemon()
    } else {
        ipc::send(&args).map_err(Into::into)
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("random-fx: {e}");
            ExitCode::FAILURE
        }
    }
}

fn daemon() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load();
    let states: Vec<_> = states::all(&config)
        .into_iter()
        .filter(|s| config.enabled(s.name()))
        .collect();
    if states.is_empty() {
        return Err("config enables no states".into());
    }

    let (tx, rx) = mpsc::channel();
    ipc::listen(tx)?;
    let (height, clock) = (config.height, config.clock);
    let sources = signals::Sources::spawn(config.palette.clone());
    let machine = machine::Machine::new(states, config, sources, rx);
    surface::run(machine, height, clock)
}
