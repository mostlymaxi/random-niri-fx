//! Events from the outside world over a unix socket, one per line:
//! `zap`, `zap #ff8800`, ...

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct Event {
    pub verb: String,
    pub args: Vec<String>,
}

impl Event {
    pub fn parse(line: &str) -> Option<Event> {
        let mut words = line.split_whitespace().map(str::to_owned);
        let verb = words.next()?;
        Some(Event {
            verb,
            args: words.collect(),
        })
    }
}

pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| "/tmp".into());
    PathBuf::from(dir).join("random-fx.sock")
}

pub fn listen(tx: Sender<Event>) -> std::io::Result<()> {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if let Some(event) = Event::parse(&line) {
                    if tx.send(event).is_err() {
                        return;
                    }
                }
            }
        }
    });
    Ok(())
}

pub fn send(words: &[String]) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket_path())?;
    writeln!(stream, "{}", words.join(" "))
}
