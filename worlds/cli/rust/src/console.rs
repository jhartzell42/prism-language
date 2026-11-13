//! The CLI World's terminal: a program's `Console.Read` and `Console.Print` places on the
//! process's own standard input and output, one line in, one line out, a terminal
//! of lines. No terminal modes, no cursor, no colour: a shell
//! script, an agent, a person with a wooden board or a teletype can stand at the other end.
//!
//! ```prism
//! tag Console.Read of Element !Text:     # each firing is one line typed, without its line end
//!     name: Text
//! tag Console.Print of Element:          # each firing is the lines to print, in order
//!     name: Text
//!     !lines: [Text]
//! ```
//!
//! The family is the program's own (as `Fs.List` is the compiler's); this World answers it
//! by those shapes.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Instant;

use prism_rtl::{Document, Path, Pod};

/// A line from the reader, or the end of the input.
pub enum Heard {
    Line(String),
    /// The wake given to [`Console::next_line`] came first.
    Timeout,
    /// The input has ended (or failed to read): no `Console.Read` fires again.
    Ended,
}

/// The terminal: an input read on a thread of its own, line by line, from the first
/// `Console.Read` on, and an output written and flushed line by line.
pub struct Console {
    input: Option<Box<dyn Read + Send>>,
    lines: Option<Receiver<String>>,
    ended: bool,
    output: Box<dyn Write>,
}

impl Console {
    /// The process's own standard input and output.
    pub fn stdio() -> Console {
        Console::new(std::io::stdin(), std::io::stdout())
    }

    pub fn new(input: impl Read + Send + 'static, output: impl Write + 'static) -> Console {
        Console { input: Some(Box::new(input)), lines: None, ended: false, output: Box::new(output) }
    }

    /// Start reading, once: a program with no `Console.Read` never touches the input.
    pub fn start(&mut self) {
        let Some(input) = self.input.take() else { return };
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(input);
            loop {
                let mut bytes = Vec::new();
                match reader.read_until(b'\n', &mut bytes) {
                    Ok(0) => return,
                    Ok(_) => {
                        if bytes.last() == Some(&b'\n') {
                            bytes.pop();
                            if bytes.last() == Some(&b'\r') {
                                bytes.pop();
                            }
                        }
                        if send.send(String::from_utf8_lossy(&bytes).into_owned()).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        eprintln!("prism: standard input failed: {e}");
                        return;
                    }
                }
            }
        });
        self.lines = Some(receive);
    }

    /// Whether a line may still come.
    pub fn reading(&self) -> bool {
        self.lines.is_some() && !self.ended
    }

    /// The next line, waiting for it until `wake` (forever with no wake).
    pub fn next_line(&mut self, wake: Option<Instant>) -> Heard {
        let Some(lines) = &self.lines else { return Heard::Ended };
        let heard = match wake {
            None => lines.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(at) => lines.recv_timeout(at.saturating_duration_since(Instant::now())),
        };
        match heard {
            Ok(line) => Heard::Line(line),
            Err(RecvTimeoutError::Timeout) => Heard::Timeout,
            Err(RecvTimeoutError::Disconnected) => {
                self.ended = true;
                Heard::Ended
            }
        }
    }

    /// Print a firing's lines, each flushed as it is written; `addressed` puts the
    /// terminal's name before each. A failed write is the World's failure.
    pub fn print(&mut self, name: &str, lines: &[String], addressed: bool) -> std::io::Result<()> {
        for line in lines {
            if addressed {
                writeln!(self.output, "{name}: {line}")?;
            } else {
                writeln!(self.output, "{line}")?;
            }
            self.output.flush()?;
        }
        Ok(())
    }
}

/// A `Console.Read` or `Console.Print` element's terminal.
pub fn terminal_of(element: &Pod) -> Option<String> {
    match element.tag()? {
        "Console.Read" | "Console.Print" => element.field("name").and_then(Pod::as_text).map(|t| t.to_string()),
        _ => None,
    }
}

/// The terminals of a document's places that `live` keeps.
pub fn terminals<'a>(places: impl Iterator<Item = (&'a Path, &'a Pod)>, live: impl Fn(&Path) -> bool) -> BTreeSet<String> {
    places.filter(|(p, _)| live(p)).filter_map(|(_, e)| terminal_of(e)).collect()
}

/// A `Console.Print` firing's lines: the `[Text]` the request carries.
pub fn lines_of(value: &Pod) -> Option<Vec<String>> {
    match value.structure() {
        Pod::Array(items) => items.iter().map(|i| i.as_text().map(|t| t.to_string())).collect(),
        _ => None,
    }
}

/// Where a line typed goes: the terminal it is for and the text that terminal hears. With
/// one terminal the line is plain; with several it is addressed `NAME: text`, the longest
/// name that fits winning. `None` for a line that names no terminal.
pub fn address(line: &str, terminals: &BTreeSet<String>) -> Option<(String, String)> {
    if terminals.len() == 1 {
        return terminals.iter().next().map(|t| (t.clone(), line.to_string()));
    }
    terminals
        .iter()
        .filter_map(|t| line.strip_prefix(t.as_str()).and_then(|rest| rest.strip_prefix(": ")).map(|rest| (t.clone(), rest.to_string())))
        .max_by_key(|(t, _)| t.len())
}

/// The live `Console.Read` places of a terminal.
pub fn readers(doc: &Document, terminal: &str, live: impl Fn(&Path) -> bool) -> Vec<Path> {
    doc.iter()
        .filter(|(p, e)| e.tag() == Some("Console.Read") && live(p) && terminal_of(e).as_deref() == Some(terminal))
        .map(|(p, _)| p.clone())
        .collect()
}
