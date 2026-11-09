# Chess: A Proof-of-Concept Example

This is a simple chess app in Prism as a proof of concept and first demo app. It's intended
simply for verifying the programming language is working and as a test case for working on
the compiler. By reading the code, you can also get a basic idea of Prism's basic syntax and semantics.

It's not an amazing UX for a chess app. White and black share a window, and have to hover
over the same computer or pass a laptop back and forth. Play is by clicking on squares.
It doesn't support computer players, network play, or any other feature beyond implementing
two-player chess in a single window on-screen. I'm not even sure the rules for chess
are correct.

It's also not a polished technical demo for Prism, just the first one. It doesn't support any of my planned
features for how you would structure a two-player board game in an actual ecosystem, like
the `^Document` trait or other such ideas (mostly unpublished as of this writing).

The code is an LLM-generated port of
[`reflex-chess`](https://github.com/jhartzell42/reflex-chess/) to Prism, which I
wrote as a personal exercise and as an example app while I was working at
[Obsidian](https://obsidian.systems/).

Better things are yet to come! This is just an initial taste.

## Trying it out

To run, from the root of the repo after following [basic set-up instructions](../../README.md):

```sh
./prism run examples/chess
```

To run unit tests:

```sh
./prism test examples/chess
```