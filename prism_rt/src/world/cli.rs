//! The command line's World: the `Fs`, `Process` and `Time` languages (see `families`),
//! over a file system that is either the real one or held in memory for tests.
//!
//! - **`Fs.Contents(path)`** answers `$~Contents` from the ready state: the file's text,
//!   or `Error(kind=Fs.NotFound, ..)`. When this World writes a file, it pushes the new
//!   contents to every `Fs.Contents` of that path in the next tick.
//! - **`Fs.Output(path, text=$Text)`** answers `$WriteState`: `Writing` as its first
//!   value; whenever the place is added or its text changes, the World writes the file
//!   (making parent directories, writing beside the target and renaming) and pushes
//!   `Written` keyed by the element in the next tick.
//! - **`Fs.File(path, requests=!FileRequest)`** answers each `Read` or `Write text` in the
//!   next tick with `~ Contents(..)` or an `Error`.
//! - **`Process.Args`** answers the arguments, plain, once.
//! - **`Process.Exit(code=!U8)`** ends the program after every request of its tick is
//!   performed, in document order (Semantics ch. 9).
//! - **`Time.After(wakes=!Wake)`** answers each wake with the same record after its delay,
//!   in a later tick; `@Later` rides on it.
//!
//! A program that never exits ends with code 0 when nothing can cause another tick: no
//! answer waiting and `pending` not set ([`drive`](super::drive)). The ready state reads
//! the file system when a place asks, which is the ready state materialised lazily
//! rather than the "no I/O in a query" of Semantics ch. 9 to the letter; the crate report
//! lists it.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::document::ReadyState;
use crate::families::fs;
use crate::tick::{Inputs, Outputs};
use crate::value::{Element, Path, Value};

use super::World;

/// What the `Fs` language reads and writes.
pub trait FileSystem {
    /// The file's text, or an `Error` value (`Error(kind=Fs.NotFound, ..)`).
    fn read(&self, path: &str) -> Result<String, Value>;
    fn write(&mut self, path: &str, text: &str) -> Result<(), Value>;
}

/// The real file system, with relative paths against a root directory.
pub struct RealFs {
    pub root: std::path::PathBuf,
}

impl RealFs {
    fn resolve(&self, path: &str) -> std::path::PathBuf {
        self.root.join(path)
    }
}

fn io_error(e: std::io::Error, path: &str) -> Value {
    let kind = match e.kind() {
        std::io::ErrorKind::NotFound => "Fs.NotFound",
        std::io::ErrorKind::PermissionDenied => "Fs.Denied",
        _ => "Fs.Failed",
    };
    Value::error(kind, &format!("{path}: {e}"))
}

impl FileSystem for RealFs {
    fn read(&self, path: &str) -> Result<String, Value> {
        std::fs::read_to_string(self.resolve(path)).map_err(|e| io_error(e, path))
    }
    /// Write beside the target, then rename, so a reader never sees half a file.
    fn write(&mut self, path: &str, text: &str) -> Result<(), Value> {
        let target = self.resolve(path);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| io_error(e, path))?;
        }
        let tmp = target.with_extension("prism-tmp");
        std::fs::write(&tmp, text).map_err(|e| io_error(e, path))?;
        std::fs::rename(&tmp, &target).map_err(|e| io_error(e, path))
    }
}

/// A file system held in memory, for tests.
#[derive(Default, Clone)]
pub struct MemFs {
    pub files: BTreeMap<String, String>,
}

impl FileSystem for MemFs {
    fn read(&self, path: &str) -> Result<String, Value> {
        self.files.get(path).cloned().ok_or_else(|| Value::error("Fs.NotFound", &format!("{path}: no such file")))
    }
    fn write(&mut self, path: &str, text: &str) -> Result<(), Value> {
        self.files.insert(path.to_string(), text.to_string());
        Ok(())
    }
}

/// The ready state: what the file system and the arguments say, asked when a place is
/// made.
struct CliReady<F: FileSystem> {
    fs: Rc<RefCell<F>>,
    args: Rc<Vec<String>>,
}

fn text_field(e: &Element, name: &str) -> Option<String> {
    e.field(name).and_then(Value::as_text).map(|t| t.to_string())
}

impl<F: FileSystem> ReadyState for CliReady<F> {
    fn first_value(&self, element: &Value) -> Option<Value> {
        let e = element.as_element()?;
        match &*e.tag {
            "Fs.Contents" => {
                let path = text_field(e, "path")?;
                Some(match self.fs.borrow().read(&path) {
                    Ok(text) => Value::ok(fs::contents_value(&text)),
                    Err(err) => err,
                })
            }
            "Fs.Output" => Some(fs::writing()),
            "Process.Args" => Some(Value::array(self.args.iter().map(|a| Value::text(a)).collect())),
            _ => None,
        }
    }
}

/// The command line's World.
pub struct CliWorld<F: FileSystem + 'static> {
    pub fs: Rc<RefCell<F>>,
    args: Rc<Vec<String>>,
    exit: Option<u8>,
    answers: Vec<(Path, Value)>,
    pushes: Vec<(Value, Value)>,
    /// Each `Fs.File` place's file, remembered from the patch that added it: its path
    /// field is identity, so it never changes while the place lives.
    file_places: BTreeMap<Path, String>,
}

impl<F: FileSystem + 'static> CliWorld<F> {
    pub fn new(fs: F, args: Vec<String>) -> CliWorld<F> {
        CliWorld {
            fs: Rc::new(RefCell::new(fs)),
            args: Rc::new(args),
            exit: None,
            answers: vec![],
            pushes: vec![],
            file_places: BTreeMap::new(),
        }
    }

    /// Write a file and push its new contents to every watch of that path.
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), Value> {
        self.fs.borrow_mut().write(path, text)?;
        let watch = Value::element(Element::plain("Fs.Contents", vec![("path", Value::text(path))]));
        self.pushes.push((watch, Value::ok(fs::contents_value(text))));
        Ok(())
    }
}

impl<F: FileSystem + 'static> World for CliWorld<F> {
    fn ready(&self) -> Rc<dyn ReadyState> {
        Rc::new(CliReady { fs: self.fs.clone(), args: self.args.clone() })
    }

    fn perform(&mut self, outputs: &Outputs) {
        // Requests first, in document order, torn-down places included.
        for r in &outputs.requests {
            match &*r.tag {
                "Process.Exit" => {
                    if self.exit.is_none() {
                        self.exit = r.value.as_i128().and_then(|c| u8::try_from(c).ok());
                    }
                }
                "Time.After" => {
                    if let Some(Value::Duration(ns)) = r.value.field("after") {
                        if *ns > 0 {
                            std::thread::sleep(std::time::Duration::from_nanos(*ns as u64));
                        }
                    }
                    self.answers.push((r.path.clone(), r.value.clone()));
                }
                "Fs.File" => {
                    let Some(file) = self.file_places.get(&r.path).cloned() else { continue };
                    let answer = match &r.value {
                        Value::Choice(c) if &*c.ctor == "Read" => match self.fs.borrow().read(&file) {
                            Ok(text) => Value::ok(fs::contents_value(&text)),
                            Err(e) => e,
                        },
                        Value::Choice(c) if &*c.ctor == "Write" => {
                            let text = c.payload.first().and_then(Value::as_text).map(|t| t.to_string()).unwrap_or_default();
                            match self.write_file(&file, &text) {
                                Ok(()) => Value::ok(fs::contents_value(&text)),
                                Err(e) => e,
                            }
                        }
                        _ => continue,
                    };
                    self.answers.push((r.path.clone(), answer));
                }
                _ => {}
            }
        }
        // Then the patch: remember `Fs.File` places, write `Fs.Output`s.
        for (path, _) in &outputs.patch.removed {
            self.file_places.remove(path);
        }
        for (path, e) in &outputs.patch.added {
            if let Some(el) = e.as_element().filter(|el| &*el.tag == "Fs.File") {
                if let Some(file) = text_field(el, "path") {
                    self.file_places.insert(path.clone(), file);
                }
            }
        }
        let writes: Vec<&Value> = outputs
            .patch
            .added
            .iter()
            .map(|(_, e)| e)
            .chain(outputs.patch.changed.iter().map(|(_, _, e)| e))
            .filter(|e| e.as_element().is_some_and(|e| &*e.tag == "Fs.Output"))
            .collect();
        for e in writes {
            let el = e.as_element().unwrap();
            let (Some(path), Some(text)) = (text_field(el, "path"), text_field(el, "text")) else { continue };
            let state = match self.write_file(&path, &text) {
                Ok(()) => fs::written(),
                Err(err) => Value::ctor("WriteFailed", vec![err]),
            };
            self.pushes.push((e.clone(), state));
        }
    }

    fn next_inputs(&mut self) -> Option<Inputs> {
        if self.answers.is_empty() && self.pushes.is_empty() {
            return None;
        }
        Some(Inputs {
            answers: std::mem::take(&mut self.answers),
            pushes: std::mem::take(&mut self.pushes),
            ready: self.ready(),
        })
    }

    fn exit_code(&self) -> Option<u8> {
        self.exit
    }
}
