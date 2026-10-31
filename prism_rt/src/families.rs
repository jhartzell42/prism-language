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
//! `Fs`"; TD-W1). `Gui` is the four tags a drawing World accepts, as the chess window
//! library declares them (Pitch chapters 5 and 6); `Cards.Deck` is the card table's deck
//! (Pitch ch. 11), which the Test World answers from a seed.

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
    //! The four tags a drawing World accepts (Pitch chapters 5 and 6), as the window
    //! library (`examples/chess/window.prism`) declares them, and the values their answers
    //! are made of. Geometry is `Float` throughout (D14).
    use super::*;
    /// `tag Gui.Window of Element ((WindowId, !WindowInput)): title: $Text, size: Extent`:
    /// a bundle, the window's id answered once by the first-value query and its inputs
    /// answered by path.
    pub fn window() -> Rc<TagDecl> {
        TagDecl::new(
            "Gui.Window",
            &[("title", Live), ("size", Identity)],
            OutputKind::Bundle(vec![OutputKind::Plain, OutputKind::Event]),
        )
    }
    /// `tag Gui.Scene of Element unit: window: WindowId, draw: $[Draw]`.
    pub fn scene() -> Rc<TagDecl> {
        TagDecl::new("Gui.Scene", &[("window", Identity), ("draw", Live)], OutputKind::Unit)
    }
    /// `tag Gui.Metrics of Element $Extent: text: Text`: the size of a text.
    pub fn metrics() -> Rc<TagDecl> {
        TagDecl::new("Gui.Metrics", &[("text", Identity)], OutputKind::Dynamic)
    }
    /// `tag Gui.Focus of Element unit: window: WindowId, at: $?Rect`.
    pub fn focus() -> Rc<TagDecl> {
        TagDecl::new("Gui.Focus", &[("window", Identity), ("at", Live)], OutputKind::Unit)
    }
    /// `WindowId "Chess"`: the World's handle for a window.
    pub fn window_id(title: &str) -> Value {
        Value::ctor("WindowId", vec![Value::text(title)])
    }
    /// `Point(x=.., y=..)`.
    pub fn point(x: f64, y: f64) -> Value {
        Value::record("Point", vec![("x", Value::float(x)), ("y", Value::float(y))])
    }
    /// `Extent(w=.., h=..)`.
    pub fn extent(w: f64, h: f64) -> Value {
        Value::record("Extent", vec![("w", Value::float(w)), ("h", Value::float(h))])
    }
    /// `Rect(x=.., y=.., w=.., h=..)`.
    pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Value {
        Value::record(
            "Rect",
            vec![("x", Value::float(x)), ("y", Value::float(y)), ("w", Value::float(w)), ("h", Value::float(h))],
        )
    }
    /// `Press Point(x=.., y=..)`, a `WindowInput`: the pointer pressed at a point.
    pub fn press(x: f64, y: f64) -> Value {
        Value::ctor("Press", vec![point(x, y)])
    }
    /// The Test World's font (README doubt TM, prism-reflex's rule): nine units a
    /// character and fourteen high, one line.
    pub fn test_font_extent(text: &str) -> Value {
        extent(9.0 * text.chars().count() as f64, 14.0)
    }
}

pub mod cards {
    //! The card table's deck (Pitch ch. 11, "The language of a card table"; Conformance
    //! ch. 1, "The seeded deck").
    use super::*;
    /// `tag Cards.Deck of Cards.Element !Card: hand: Nat, draws: !unit`: a shuffled deck,
    /// one card per request; a new `hand`, a new shuffle.
    pub fn deck() -> Rc<TagDecl> {
        TagDecl::new("Cards.Deck", &[("hand", Identity), ("draws", Request)], OutputKind::Event)
    }
    /// The suits in the order of the deck before shuffling.
    pub const SUITS: [&str; 4] = ["Clubs", "Diamonds", "Hearts", "Spades"];
    /// The ranks, Two to Ace.
    pub const RANKS: [&str; 13] =
        ["Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Jack", "Queen", "King", "Ace"];
    /// `Card(rank=.., suit=..)`.
    pub fn card(rank: &str, suit: &str) -> Value {
        Value::record("Card", vec![("rank", Value::ctor(rank, vec![])), ("suit", Value::ctor(suit, vec![]))])
    }
    /// The deck before shuffling: position 0 the two of clubs, 51 the ace of spades.
    pub fn unshuffled() -> Vec<Value> {
        SUITS.iter().flat_map(|s| RANKS.iter().map(move |r| card(r, s))).collect()
    }
}
