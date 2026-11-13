//! The `Net` family as the CLI World answers it (the standard library's `Documents`,
//! "a protocol that crosses a wire"): a document's messages over TCP, one message per line,
//! each in the POD printed form (Type System ch. 2), so a phone and a Hub are two programs.
//!
//! - **`Net.Link(to=Address, !sends=[Message])`** connects to the address when the place is
//!   added; each firing's messages are written, and it answers `![Message]` with the
//!   messages that arrived since the last tick.
//! - **`Net.Listen(port)`** listens on the port when the place is added and answers
//!   `!Connection` once per connection that opens, in the order they open.
//! - **`Net.Channel(connection, !sends=[Message])`** stands for one open connection: its
//!   firings are written to it, and it answers `![Message]` with what arrived.
//!
//! A place that leaves closes what it opened. A connection that fails is dropped with a
//! line on standard error; its places hear nothing more.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use std::collections::BTreeMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};

use prism_rtl::{Path, Pod};

/// One open stream and the bytes read from it that make no whole line yet.
struct Stream {
    socket: TcpStream,
    pending: Vec<u8>,
}

impl Stream {
    fn new(socket: TcpStream) -> Option<Stream> {
        socket.set_nonblocking(true).ok()?;
        Some(Stream { socket, pending: Vec::new() })
    }

    /// Write each message as one line; false when the stream failed.
    fn send(&mut self, messages: &[Pod]) -> bool {
        for m in messages {
            let mut line = prism_rtl::print::show(m);
            line.push('\n');
            if self.socket.write_all(line.as_bytes()).is_err() {
                return false;
            }
        }
        true
    }

    /// The whole lines that arrived, each read back as a message; `None` when the stream
    /// closed or failed.
    fn receive(&mut self) -> Option<Vec<Pod>> {
        let mut buf = [0u8; 4096];
        loop {
            match self.socket.read(&mut buf) {
                Ok(0) => return None,
                Ok(n) => self.pending.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(_) => return None,
            }
        }
        let mut out = Vec::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let text = String::from_utf8_lossy(&line[..line.len() - 1]).to_string();
            match prism_rtl::read::value(&text) {
                Ok(pod) => out.push(pod),
                Err(e) => eprintln!("prism: a line on the wire is no message: {e}"),
            }
        }
        Some(out)
    }
}

/// What the World holds for the `Net` places in the document.
#[derive(Default)]
pub struct Net {
    listeners: BTreeMap<Path, TcpListener>,
    links: BTreeMap<Path, Stream>,
    /// The connections a listener opened, by their number.
    connections: BTreeMap<u64, Stream>,
    /// Each `Net.Channel` place and the connection it stands for.
    channels: BTreeMap<Path, u64>,
    next: u64,
}

fn field_u64(e: &Pod, name: &str) -> Option<u64> {
    e.field(name).and_then(|p| p.as_f64()).map(|f| f as u64)
}

impl Net {
    /// Whether any socket is open, so that the World waits for the wire.
    pub fn open(&self) -> bool {
        !self.listeners.is_empty() || !self.links.is_empty() || !self.connections.is_empty()
    }

    /// A place added to the document.
    pub fn added(&mut self, path: &Path, element: &Pod) {
        match element.tag() {
            Some("Net.Listen") => {
                let Some(port) = field_u64(element, "port") else { return };
                match TcpListener::bind(("0.0.0.0", port as u16)) {
                    Ok(l) if l.set_nonblocking(true).is_ok() => {
                        self.listeners.insert(path.clone(), l);
                    }
                    _ => eprintln!("prism: cannot listen on port {port}"),
                }
            }
            Some("Net.Link") => {
                let Some(to) = element.field("to") else { return };
                let host = to.field("host").and_then(Pod::as_text).map(|t| t.to_string()).unwrap_or_default();
                let port = field_u64(to, "port").unwrap_or(0) as u16;
                match TcpStream::connect((host.as_str(), port)).ok().and_then(Stream::new) {
                    Some(s) => {
                        self.links.insert(path.clone(), s);
                    }
                    None => eprintln!("prism: cannot connect to {host}:{port}"),
                }
            }
            Some("Net.Channel") => {
                let Some(id) = element.field("connection").and_then(|c| field_u64(c, "id")) else { return };
                self.channels.insert(path.clone(), id);
            }
            _ => {}
        }
    }

    /// A place removed from the document: what it opened closes.
    pub fn removed(&mut self, path: &Path) {
        self.listeners.remove(path);
        self.links.remove(path);
        if let Some(id) = self.channels.remove(path) {
            self.connections.remove(&id);
        }
    }

    /// A firing of a place's `!sends`.
    pub fn send(&mut self, element: &Pod, path: &Path, value: &Pod) {
        let messages: Vec<Pod> = match value.structure() {
            Pod::Array(xs) => xs.clone(),
            other => vec![other.clone()],
        };
        match element.tag() {
            Some("Net.Link") => {
                if let Some(s) = self.links.get_mut(path)
                    && !s.send(&messages)
                {
                    eprintln!("prism: a link failed");
                    self.links.remove(path);
                }
            }
            Some("Net.Channel") => {
                if let Some(id) = self.channels.get(path).copied()
                    && let Some(s) = self.connections.get_mut(&id)
                    && !s.send(&messages)
                {
                    eprintln!("prism: a connection failed");
                    self.connections.remove(&id);
                }
            }
            _ => {}
        }
    }

    /// What the wire brought since the last tick: each connection opened, as an answer to its
    /// listener, and each place's messages.
    pub fn poll(&mut self) -> Vec<(Path, Pod)> {
        let mut out = Vec::new();
        for (path, l) in &self.listeners {
            if let Ok((socket, _)) = l.accept()
                && let Some(s) = Stream::new(socket)
            {
                let id = self.next;
                self.next += 1;
                self.connections.insert(id, s);
                out.push((path.clone(), Pod::record("Connection", vec![("id", Pod::int(id as i64))])));
            }
        }
        let mut gone = Vec::new();
        for (path, s) in self.links.iter_mut() {
            match s.receive() {
                Some(ms) if !ms.is_empty() => out.push((path.clone(), Pod::Array(ms))),
                Some(_) => {}
                None => gone.push(path.clone()),
            }
        }
        for p in gone {
            self.links.remove(&p);
        }
        let mut closed = Vec::new();
        for (path, id) in &self.channels {
            let Some(s) = self.connections.get_mut(id) else { continue };
            match s.receive() {
                Some(ms) if !ms.is_empty() => out.push((path.clone(), Pod::Array(ms))),
                Some(_) => {}
                None => closed.push(*id),
            }
        }
        for id in closed {
            self.connections.remove(&id);
        }
        out
    }
}
