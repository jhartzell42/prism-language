# Prism

**NOTE:** This is not a repo ready to use for any purpose, including basic
demonstrations. I'm making it public because I can, and because that way I can
share with friends, but not because it is ready for any real-world use.

Prism is a planned programming language that will be a Reflex-style FRP framework
as its own programming language. All primitive state will be expressed through dynamics
and events.

Currently, I'm prototyping the runtime in Rust. You can't write a Prism program yet.

`LANGUAGE.md` contains a pitch for what I want the programming language to look
like someday, and why I'm writing it. I'm making my progress available under the
MIT license, available in `LICENSE.md`.
## Building and running (claude-self-hosting branch)

- **`make bootstrap`**: builds the self-hosting compiler `prismc` (written in Prism) and
  checks its fixed point; installs `target/bootstrap/prismc`.
- **`make chess`**: compiles and opens the chess example (`./prism run examples/chess`).
- **`make test`**: every test: cargo's, every compiled transcript, the checker's, and the
  bootstrap.
- **`./prism run DIR`**, **`./prism test DIR`**, **`./prism check DIR`**: the driver; `run`
  picks the World from the root's `emit` row.

Each directory has a README: `prismc/` (the compiler), `prelude/`, `runtime/rust/` and
`runtime/cpp/` (the runtimes), `worlds/` (the CLI and canvas Worlds), `bootstrap/`,
`tests/`, `examples/chess/`, `docs/`.
