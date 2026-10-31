//! Helpers for building programs by hand against the Rust API, the way an interpreter
//! would: there is no parser yet.

#![allow(dead_code)]

use std::rc::Rc;

use prism_rt::families::{fs, ui};
use prism_rt::{Build, Closure, Element, Fault, TagDecl, Value};

/// Emit an element of a declared tag.
pub fn emit(b: &mut Build, decl: Rc<TagDecl>, fields: Vec<(&str, Value)>) -> Result<Value, Fault> {
    b.emit(Element::new(&decl, fields)?)
}

/// `Ui.Button(label=$"..")`.
pub fn button(b: &mut Build, label: &str) -> Result<Value, Fault> {
    emit(b, ui::button(), vec![("label", Value::text(label))])
}

/// `Ui.Label(text=..)`, with a live or plain text.
pub fn label(b: &mut Build, text: Value) -> Result<Value, Fault> {
    emit(b, ui::label(), vec![("text", text)])
}

/// `Fs.Contents(path="..")`.
pub fn contents(b: &mut Build, path: &str) -> Result<Value, Fault> {
    emit(b, fs::contents(), vec![("path", Value::text(path))])
}

pub fn int(v: &Value) -> i128 {
    v.as_i128().expect("an integer")
}

/// A closure over integers: `f(a, b, ..)`. An argument that is not an integer (an
/// event's `click`) reads as 0.
pub fn ints(name: &str, f: impl Fn(&[i128]) -> i128 + 'static) -> Closure {
    Closure::pure(name, move |a| {
        let xs: Vec<i128> = a.iter().map(|v| v.as_i128().unwrap_or(0)).collect();
        Ok(Value::int(f(&xs) as i64))
    })
}

/// A closure that formats its arguments into a text.
pub fn text(name: &str, f: impl Fn(&[Value]) -> String + 'static) -> Closure {
    Closure::pure(name, move |a| Ok(Value::text(&f(a))))
}

/// The plain text inside a value, as interpolation shows it: a text without quotes,
/// anything else in the transcript's spelling.
pub fn show(v: &Value) -> String {
    match v {
        Value::Text(t) => t.to_string(),
        other => other.to_string(),
    }
}

/// `ShowResponse` of Semantics ch. 8: a `~Contents` to a text.
pub fn show_response(v: &Value) -> String {
    match v {
        Value::Fallible(Ok(c)) => c.field("text").map(show).unwrap_or_default(),
        Value::Fallible(Err(e)) => match e.field("kind") {
            Some(Value::Choice(k)) if &*k.ctor == "Fs.NotFound" => "(no such file)".into(),
            _ => "(cannot read)".into(),
        },
        other => format!("(not a response: {other})"),
    }
}
