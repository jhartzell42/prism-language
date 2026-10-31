//! Tag declarations of the element languages the Worlds on this branch answer, and the
//! values their answers are made of. They are libraries, not the prelude (Semantics
//! Appendix B, "Prelude tags"); compiled programs and the interpreter take them from
//! here so that every World and test agrees on the fields and outputs.
//!
//! The spellings follow the books where they give one: `Ui.Button`, `Ui.Label`,
//! `Ui.TextInput` (Semantics ch. 8), `Fs.Contents` answering `$~Contents` (ch. 8; the
//! `Loading` variant of TD-W2 is not taken), `Fs.File` with `Read` and `Write` requests
//! (Conformance ch. 1), `Fs.Output(path, text=$Text)` answered `$WriteState`
//! (prism-reflex ch. 6), `Process.Exit(code=!U8)` (Platform ch. 1), `Process.Args`
//! answered plain, `Time.After(wakes=!Wake)` (Pitch ch. 6), `Test.Log` (Semantics ch. 7).
//! There is no directory listing in `Fs` (prism-reflex ch. 6, "No directory listing in
//! `Fs`"; TD-W1).

use std::rc::Rc;

use crate::value::{FieldKind::*, OutputKind, TagDecl, Value};

pub mod ui {
    use super::*;
    /// `tag Ui.Button of Element !Click: label: $Text`.
    pub fn button() -> Rc<TagDecl> {
        TagDecl::new("Ui.Button", &[("label", Live)], OutputKind::Event)
    }
    /// `tag Ui.Label of Element: text: $Text`.
    pub fn label() -> Rc<TagDecl> {
        TagDecl::new("Ui.Label", &[("text", Live)], OutputKind::Unit)
    }
    /// `tag Ui.TextInput of Element !Key: name: Text, text: $Text`.
    pub fn text_input() -> Rc<TagDecl> {
        TagDecl::new("Ui.TextInput", &[("name", Identity), ("text", Live)], OutputKind::Event)
    }
    /// `tag Ui.Image of Element: src: Text`.
    pub fn image() -> Rc<TagDecl> {
        TagDecl::new("Ui.Image", &[("src", Identity)], OutputKind::Unit)
    }
}

pub mod fs {
    use super::*;
    /// `tag Fs.Contents of Element $~Contents: path: Text`: a file's state.
    pub fn contents() -> Rc<TagDecl> {
        TagDecl::new("Fs.Contents", &[("path", Identity)], OutputKind::Dynamic)
    }
    /// `tag Fs.File of Element !~Contents: path: Text, requests: !FileRequest`: one-shot
    /// reads and writes.
    pub fn file() -> Rc<TagDecl> {
        TagDecl::new("Fs.File", &[("path", Identity), ("requests", Request)], OutputKind::Event)
    }
    /// `tag Fs.Output of Element $WriteState: path: Text, text: $Text`: a file whose text
    /// is the field's; the World writes it whenever the text changes.
    pub fn output() -> Rc<TagDecl> {
        TagDecl::new("Fs.Output", &[("path", Identity), ("text", Live)], OutputKind::Dynamic)
    }
    /// `Contents(text=..)`.
    pub fn contents_value(text: &str) -> Value {
        Value::record("Contents", vec![("text", Value::text(text))])
    }
    /// `Read`, a `Fs.File` request.
    pub fn read() -> Value {
        Value::ctor("Read", vec![])
    }
    /// `Write text`, a `Fs.File` request.
    pub fn write(text: &str) -> Value {
        Value::ctor("Write", vec![Value::text(text)])
    }
    /// `Writing`, a `WriteState`: the World has not written this text yet.
    pub fn writing() -> Value {
        Value::ctor("Writing", vec![])
    }
    /// `Written`, a `WriteState`.
    pub fn written() -> Value {
        Value::ctor("Written", vec![])
    }
}

pub mod process {
    use super::*;
    /// `tag Process.Exit of Element: code: !U8` (Platform ch. 1, Exit codes).
    pub fn exit() -> Rc<TagDecl> {
        TagDecl::new("Process.Exit", &[("code", Request)], OutputKind::Unit)
    }
    /// `tag Process.Args of Element [Text]`: plain, answered once when the place is made.
    pub fn args() -> Rc<TagDecl> {
        TagDecl::new("Process.Args", &[], OutputKind::Plain)
    }
}

pub mod time {
    use super::*;
    /// `tag Time.After of Element !Wake: wakes: !Wake`: after at least the wake's delay,
    /// the World answers with the same record.
    pub fn after() -> Rc<TagDecl> {
        TagDecl::new("Time.After", &[("wakes", Request)], OutputKind::Event)
    }
}

pub mod test {
    use super::*;
    /// `tag Test.Log of Element: requests: !Text`: a log line, recorded by the Test World.
    pub fn log() -> Rc<TagDecl> {
        TagDecl::new("Test.Log", &[("requests", Request)], OutputKind::Unit)
    }
}

pub mod gui {
    use super::*;
    /// `tag Gui.Window of Element !Press: title: Text, scene: $[Draw]`: the canvas World's
    /// window, reduced to what the Test World needs to resolve a click by path (D9).
    pub fn window() -> Rc<TagDecl> {
        TagDecl::new("Gui.Window", &[("title", Identity), ("scene", Live)], OutputKind::Event)
    }
    /// `Press(x=.., y=..)`, a window's answer.
    pub fn press(x: i64, y: i64) -> Value {
        Value::record("Press", vec![("x", Value::int(x)), ("y", Value::int(y))])
    }
}
