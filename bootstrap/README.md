# bootstrap: building prismc from nothing but Rust

`prismc` is written in Prism, so building it needs a working `prismc`. The repository
keeps no binary; it keeps one archive.

- **`prismc-rust.tar.xz`** is the seed: the Rust crate `prismc` emitted for itself at its
  last fixed point (its `src/` and `Cargo.toml`), compressed. It is the only compiled
  code in the repository, kept apart from the source.
- **`bootstrap.sh`** starts from `$PRISMC`, else the installed `target/bootstrap/prismc`,
  else the seed unpacked and built with cargo. That compiler compiles `prismc/`; cargo
  builds the result; the new binary compiles `prismc/` again; the two emitted crates must
  be byte-equal (the fixed point). It installs the result as `target/bootstrap/prismc`,
  the binary `./prism` uses.
- **`bootstrap.sh --update`** also rewrites the seed from the new fixed point, after a
  change to the compiler.
- **From the repository root:** `make bootstrap`, `make bootstrap-update`.
