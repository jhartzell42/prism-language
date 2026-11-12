# Prism

This is a self-hosting prototype for the Prism programming language. As of now, this is an
LLM-based prototype, so there might be a lot of weird stuff going on in here.

It compiles Prism through Rust.
That is to say that a compiler written in Prism translates
Prism to Rust, and then the Rust compiler translates that Rust into machine
code. The compiler can bootstrap, and in [`bootstrap/`](bootstrap/) there are
archives of the Rust translation of a compiler for bootstrapping purposes.

I'm working on a human-based rewrite. I ([Jimmy Hartzell](https://www.thecodedmessage.com/),
notoriously a human) am currently writing this `README.md` document as one of the first steps towards this
human-based rewrite. When the rewrite is complete... it will still be a
prototype, but I hope a much more legible one. I'll then probably do another
round of LLM-based experimentation on it, before another round of human refactoring and
reimplementing, before I'll have something I'm proud enough of to actually demo.

`LANGUAGE.md` contains an (obsolete) pitch for Prism, including my initial
motivations for writing it. I'm making my progress available under the MIT
license, available in `LICENSE.md`.

## Building and running

```sh
make bootstrap                # Get bootstrapping version
./prism run examples/chess    # Run chess
make chess                    # Also run chess
./prism test examples/chess   # Run unit tests for chess
make test                     # Run overly thorough test suite
```

Many subdirectories have their own `README.md` files, which contain more information.