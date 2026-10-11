//! random-fx: a line that lives on the edges of the screen. Run without
//! arguments for the daemon; see `USAGE` for the commands.

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

const USAGE: &str = "\
usage: random-fx [command]

Without a command, runs the daemon. Commands go to the running daemon and
play at the start of the next sweep:

    zap [#rrggbb]   race around the screen edges and collide bottom-right
    chime           the half-hourly clock chime, right now";

/// Commands the daemon understands, by first word.
const COMMANDS: [&str; 2] = ["zap", "chime"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None => daemon(),
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some(verb) if COMMANDS.contains(&verb) => ipc::send(&args)
            .map_err(|e| format!("can't reach the daemon ({e}); is `random-fx` running?").into()),
        Some(other) => Err(format!("unknown command `{other}`\n\n{USAGE}").into()),
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
