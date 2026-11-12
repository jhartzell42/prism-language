# The Prism table under the canvas World, Rust, built for the benchmark.
#
#   make            the driver
#   make rust       target/bench/prism-rs/driver: app.prism, with examples/basic-chess's window
#                   library, compiled to Rust, cargo release (opt-level 3)
#   ./check.sh      built, and run once with `check`
#
# The emitted main.rs has its `main` renamed and driver.rs included at its end, whose `main`
# runs the root under Hooks.
#
# Written by Claude on the claude-self-hosting branch; subject to change.

REPO := $(abspath ../../..)

.PHONY: all rust FORCE
all: rust

# The program, emitted afresh whenever the sources or the installed prismc are newer than
# its main.rs.
INSTALLED_PRISMC := $(wildcard $(REPO)/target/bootstrap/prismc)

RS := $(REPO)/target/bench/prism-rs
RSBUILD := $(REPO)/target/bench/prism-rs-build

rust: $(RS)/driver

$(RS)/src/main.rs: app.prism $(wildcard $(REPO)/examples/basic-chess/basic_window/*.prism) driver.rs $(INSTALLED_PRISMC)
	cd $(REPO) && ./prism compile bench/gui/prism examples/basic-chess/basic_window -o target/bench/prism-rs
	sed -i.emitted 's/^fn main() {$$/fn prism_emitted_main() {/' $@
	echo 'include!("$(abspath driver.rs)");' >> $@

$(RS)/driver: $(RS)/src/main.rs FORCE
	CARGO_TARGET_DIR=$(RSBUILD) cargo build --offline --release -q --manifest-path $(RS)/Cargo.toml
	cp $(RSBUILD)/release/prism $@

FORCE:
