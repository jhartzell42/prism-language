//! The document and the patch diff (Semantics ch. 8, "The patch"; Graph ch. 4, "The
//! patch pass and the request list").
//!
//! A program's patch pass writes [`Edit`]s: a place built, torn down, or possibly
//! changed. [`Document::diff`] turns them into the tick's [`Patch`] against the document
//! last sent, so `Changed` goes out only for a place whose `Level` really moved, a torn-down
//! place comes with its element as it stood after the previous tick, and every list is in
//! path order. A World keeps its own copy of the document with [`Document::apply`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use crate::pod::{Path, Pod};

/// What the patch pass says about one place.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    Added(Path, Pod),
    /// The place's `Level` after the tick; the diff drops it if nothing moved.
    Changed(Path, Pod),
    Removed(Path),
}

/// One tick's change to the document, each list in path order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    pub added: Vec<(Path, Pod)>,
    pub changed: Vec<(Path, Pod)>,
    /// With the element as it stood after the previous tick.
    pub removed: Vec<(Path, Pod)>,
}

impl Patch {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// A request that fired: a place's path, the request field's name when its tag has more
/// than one, and the field's `Edge` value.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub path: Path,
    pub field: Option<Rc<str>>,
    pub value: Pod,
}

impl fmt::Display for Request {
    /// The transcript's `requested:` line: `path: value`, or `path: field=value`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.field {
            Some(name) => write!(f, "{}: {}={}", self.path, name, self.value),
            None => write!(f, "{}: {}", self.path, self.value),
        }
    }
}

/// Each place's `Level`, by path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document(BTreeMap<Path, Pod>);

impl Document {
    pub fn new() -> Document {
        Document::default()
    }

    /// The tick's patch from the patch pass's edits, applied to this document. A path
    /// torn down and built again in one tick (a `for` key removed and added back) is two
    /// widgets: `Removed` and `Added`, never `Changed`.
    pub fn diff(&mut self, edits: Vec<Edit>) -> Patch {
        let mut patch = Patch::default();
        let mut changed: BTreeMap<Path, Pod> = BTreeMap::new();
        let mut born: BTreeSet<Path> = BTreeSet::new();
        for edit in edits {
            match edit {
                Edit::Removed(path) => {
                    if let Some(old) = self.0.remove(&path) {
                        changed.remove(&path);
                        patch.removed.push((path, old));
                    }
                }
                Edit::Added(path, level) => {
                    born.insert(path.clone());
                    self.0.insert(path.clone(), level.clone());
                    patch.added.push((path, level));
                }
                Edit::Changed(path, level) => {
                    if born.contains(&path) || self.0.get(&path) == Some(&level) || !self.0.contains_key(&path) {
                        continue;
                    }
                    self.0.insert(path.clone(), level.clone());
                    changed.insert(path, level);
                }
            }
        }
        patch.changed = changed.into_iter().collect();
        patch.added.sort_by(|a, b| a.0.cmp(&b.0));
        patch.removed.sort_by(|a, b| a.0.cmp(&b.0));
        patch
    }

    /// A World's side: the document after a patch.
    pub fn apply(&mut self, patch: &Patch) {
        for (p, _) in &patch.removed {
            self.0.remove(p);
        }
        for (p, e) in patch.added.iter().chain(&patch.changed) {
            self.0.insert(p.clone(), e.clone());
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Pod> {
        self.0.get(path)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&Path, &Pod)> {
        self.0.iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Every element in the document at any depth.
    pub fn elements(&self) -> Vec<&Pod> {
        let mut out = Vec::new();
        self.0.values().for_each(|l| l.elements(&mut out));
        out
    }
}
