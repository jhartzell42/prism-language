//! The features `docs/decisions.md` ("Features in and out") puts out of this compiler, and
//! where the parser names each.
//!
//! "Out for this compiler, each rejected with a diagnostic naming the feature, never
//! silently." The parser accepts each construct into the tree, so a program that uses one
//! still parses and prints, and records a [`crate::Diagnostic`] with
//! [`crate::Stage::OutOfScope`] whose `feature` is the name below.
//!
//! Some features on the list are not syntax and cannot be seen by a parser: live update,
//! checkpoints, the Prism World, `Duration` arithmetic (a duration literal is in; adding two
//! is a checker's question). Those are left to the checker in `prismc`, and the table says
//! so.

/// One out-of-scope feature: its name as diagnostics carry it, and what the parser
/// detects.
pub struct Feature {
    pub name: &'static str,
    pub detected_by: &'static str,
}

pub const OUT_OF_SCOPE: &[Feature] = &[
    Feature { name: "`T monad:` blocks and `^Monad`", detected_by: "`T monad:` blocks, `return` statements, the trait reference `^Monad`" },
    Feature { name: "higher-kinded traits", detected_by: "a trait whose parameter is applied to arguments in its items' types (`f a`)" },
    Feature { name: "sized ints other than `U8`", detected_by: "the type names U16..U128 and I8..I128" },
    Feature { name: "`Instant`", detected_by: "the type name `Instant` (`Duration` arithmetic is the checker's to refuse)" },
    Feature { name: "sparse tuples", detected_by: "an absent component `-` in a tuple, pattern or type" },
    Feature { name: "`!Align`", detected_by: "the name `!Align`" },
    Feature { name: "non-empty arrays", detected_by: "the non-empty mark `+` after `]]`, `}` or a type's `]`" },
    Feature { name: "suffixed operators", detected_by: "an operator touching an uppercase name, `>Int`" },
    Feature { name: "named lenses `&Home` and `Over`", detected_by: "`&Name` in a signature or definition, `&Name &ref` in an `update` statement, the name `Over`" },
    Feature { name: "POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`)", detected_by: "those four names" },
    Feature { name: "the template notation", detected_by: "a `<markdown>` or `<html>` template under `output` or `inline_text`, and a file whose first token is a language tag" },
    Feature { name: "indexed choices/GADTs for user types", detected_by: "a choice alternative written `Name: T -> R`" },
    Feature { name: "the Prism World", detected_by: "not syntax: the checker's" },
    Feature { name: "live update", detected_by: "not syntax: the checker's" },
    Feature { name: "checkpoints", detected_by: "not syntax: the checker's" },
];
