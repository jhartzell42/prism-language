//! What the `Fs` language reads, writes and lists: the real file system, or one held in
//! memory for tests.
//!
//! - Paths are relative to a root (the working directory under `prism run`), with `/`
//!   between their parts, as the program wrote them.
//! - A write goes beside the target and is renamed into place, so a reader never sees half
//!   a file; parent directories are made.
//! - A listing is every `.prism` and `.prismtest` file under a directory, at any depth,
//!   each path the directory as written joined to the file's path inside it, sorted as
//!   text. Entries whose name starts with `.` are skipped, and so are symbolic links to
//!   directories, so a listing always ends.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use prism_rtl::Error;

/// What the `Fs` language reads, writes and lists.
pub trait FileSystem {
    /// The file's text, or an `Error` of kind `Fs.NotFound`, `Fs.Denied` or `Fs.Failed`.
    fn read(&self, path: &str) -> Result<String, Error>;
    fn write(&mut self, path: &str, text: &str) -> Result<(), Error>;
    /// The `.prism` and `.prismtest` files under `dir`, sorted. A directory that is
    /// missing or unreadable lists nothing: `Fs.List` answers `$[Text]`, with no `~`.
    fn list(&self, dir: &str) -> Vec<String>;
}

/// Whether a listing includes this file.
pub fn listed(name: &str) -> bool {
    name.ends_with(".prism") || name.ends_with(".prismtest")
}

/// `dir` and a path inside it, as a listing spells it.
fn joined(dir: &str, rest: &str) -> String {
    match dir.trim_end_matches('/') {
        "" | "." if !dir.starts_with('/') => rest.to_string(),
        d => format!("{d}/{rest}"),
    }
}

/// The real file system, with relative paths against a root directory.
pub struct RealFs {
    pub root: PathBuf,
}

fn io_error(e: std::io::Error, path: &str) -> Error {
    let kind = match e.kind() {
        std::io::ErrorKind::NotFound => "Fs.NotFound",
        std::io::ErrorKind::PermissionDenied => "Fs.Denied",
        _ => "Fs.Failed",
    };
    Error::new(kind, format!("{path}: {e}"))
}

impl RealFs {
    fn walk(at: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(at) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { continue };
            if name.starts_with('.') {
                continue;
            }
            let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            if kind.is_dir() {
                RealFs::walk(&entry.path(), &rel, out);
            } else if listed(&name) && (kind.is_file() || entry.path().is_file()) {
                out.push(rel);
            }
        }
    }
}

impl FileSystem for RealFs {
    fn read(&self, path: &str) -> Result<String, Error> {
        std::fs::read_to_string(self.root.join(path)).map_err(|e| io_error(e, path))
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), Error> {
        let target = self.root.join(path);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| io_error(e, path))?;
        }
        let mut tmp = target.clone().into_os_string();
        tmp.push(".prism-tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &target)).map_err(|e| io_error(e, path))
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let mut rel = Vec::new();
        RealFs::walk(&self.root.join(dir), "", &mut rel);
        let mut out: Vec<String> = rel.iter().map(|r| joined(dir, r)).collect();
        out.sort();
        out
    }
}

/// A file system held in memory, for tests: each file by its path.
#[derive(Default, Clone)]
pub struct MemFs {
    pub files: BTreeMap<String, String>,
}

impl FileSystem for MemFs {
    fn read(&self, path: &str) -> Result<String, Error> {
        self.files.get(path).cloned().ok_or_else(|| Error::new("Fs.NotFound", format!("{path}: no such file")))
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), Error> {
        self.files.insert(path.to_string(), text.to_string());
        Ok(())
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let d = dir.trim_end_matches('/');
        let inside = |p: &str| match d {
            "" | "." => true,
            d => p.strip_prefix(d).is_some_and(|r| r.starts_with('/')),
        };
        let mut out: Vec<String> = self
            .files
            .keys()
            .filter(|p| inside(p) && p.rsplit('/').next().is_some_and(|n| listed(n) && !n.starts_with('.')))
            .cloned()
            .collect();
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listing_joins_the_directory_as_written() {
        assert_eq!(joined("examples", "a.prism"), "examples/a.prism");
        assert_eq!(joined("examples/", "a.prism"), "examples/a.prism");
        assert_eq!(joined(".", "a.prism"), "a.prism");
        assert_eq!(joined("/", "a.prism"), "/a.prism");
    }

    #[test]
    fn a_memory_listing_takes_only_what_is_under_the_directory() {
        let mut fs = MemFs::default();
        for p in ["ex/b.prism", "ex/a/c.prismtest", "ex/a/c.expected", "exx/d.prism", "top.prism"] {
            fs.write(p, "").unwrap();
        }
        assert_eq!(fs.list("ex"), vec!["ex/a/c.prismtest", "ex/b.prism"]);
        assert_eq!(fs.list("."), vec!["ex/a/c.prismtest", "ex/b.prism", "exx/d.prism", "top.prism"]);
        assert!(fs.list("nowhere").is_empty());
    }
}
