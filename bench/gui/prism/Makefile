# The Prism table under the canvas World, C++ and Rust, built for the benchmark (bench/gui/README.md).
#
#   make            both drivers
#   make cpp        target/bench/prism/driver: bench.prism, with examples/chess's window
#                   library, compiled to C++ by the prism script's prismc, at -O2
#   make rust       target/bench/prism-rs/driver: the same compiled to Rust, cargo
#                   release (opt-level 3)
#
# The emitted main.cpp holds the crate's own definitions, so it is compiled here with its
# `main` renamed and linked beside driver.cpp, whose `main` runs the root under Hooks. The
# emitted main.rs gets the same: its `main` renamed and driver.rs included at its end.
#
# Written by Claude on the claude-self-hosting branch; subject to change.

REPO := $(abspath ../../..)
OUT := $(REPO)/target/bench/prism
SRC := $(OUT)/src
CXX ?= clang++
OPT ?= -O2
CONFIG := bench-$(notdir $(CXX))$(OPT)
RT := $(REPO)/runtime/cpp
CLI := $(REPO)/worlds/cli-cpp
CANVAS := $(REPO)/worlds/canvas-cpp
RTLIB := $(RT)/build/$(CONFIG)/libprism_rtl.a
CANVASLIB := $(CANVAS)/build/$(CONFIG)/libprism_world_canvas.a
ifeq ($(shell uname -s),Darwin)
SYS_LIBS := $(shell pkg-config --libs cairo freetype2) -framework Cocoa -framework QuartzCore
else
SYS_LIBS := $(shell pkg-config --libs cairo freetype2 x11 xext)
endif
FLAGS := -std=gnu++20 $(OPT) -I$(RT)/include -I$(CLI)/include -I$(CANVAS)/include $(shell pkg-config --cflags cairo freetype2) -I$(SRC) -Wall -Wno-unused -Wno-unused-parameter -Wno-unused-label
BUILD := $(OUT)/build-driver

.PHONY: all cpp rust emitted FORCE
all: cpp rust
cpp: $(OUT)/driver

# The program, emitted afresh whenever the sources are newer than its main.cpp.
$(SRC)/main.cpp: bench.prism $(REPO)/examples/chess/window.prism
	cd $(REPO) && ./prism compile --target cpp bench/prism/bench.prism examples/chess/window.prism -o target/bench/prism

$(RTLIB): FORCE
	@$(MAKE) -s -C $(RT) lib CXX=$(CXX) CONFIG=$(CONFIG) OPT="$(OPT)"

$(CANVASLIB): FORCE | $(RTLIB)
	@$(MAKE) -s -C $(CANVAS) lib CXX=$(CXX) CONFIG=$(CONFIG) OPT="$(OPT)"

# Every emitted source but main.cpp, compiled as the emitted Makefile would.
emitted: $(SRC)/main.cpp
	@$(MAKE) -s -f $(abspath Makefile) objects

PROG_SRCS = $(filter-out $(SRC)/main.cpp,$(wildcard $(SRC)/*.cpp))
PROG_OBJS = $(PROG_SRCS:$(SRC)/%.cpp=$(BUILD)/%.o)

.PHONY: objects
objects: $(PROG_OBJS) $(BUILD)/emitted_main.o $(BUILD)/driver.o

$(BUILD)/%.o: $(SRC)/%.cpp $(SRC)/crate.hpp $(SRC)/prelude.hpp
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -c $< -o $@

$(BUILD)/emitted_main.o: $(SRC)/main.cpp $(SRC)/crate.hpp
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -Dmain=prism_emitted_main -c $< -o $@

$(BUILD)/driver.o: driver.cpp $(SRC)/crate.hpp
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -c $< -o $@

# The objects are globbed when the link runs: on a clean build none exist when make reads
# this file, before prismc has emitted their sources.
$(OUT)/driver: emitted $(CANVASLIB) $(RTLIB)
	$(CXX) $(FLAGS) -o $@ $(BUILD)/*.o $(CANVASLIB) $(RTLIB) $(SYS_LIBS)

# ---- Rust ---------------------------------------------------------------------------

RS := $(REPO)/target/bench/prism-rs
RSBUILD := $(REPO)/target/bench/prism-rs-build

rust: $(RS)/driver

$(RS)/src/main.rs: bench.prism $(REPO)/examples/chess/window.prism driver.rs
	cd $(REPO) && ./prism compile bench/prism/bench.prism examples/chess/window.prism -o target/bench/prism-rs
	sed -i.emitted 's/^fn main() {$$/fn prism_emitted_main() {/' $@
	echo 'include!("$(abspath driver.rs)");' >> $@

$(RS)/driver: $(RS)/src/main.rs FORCE
	cd $(RS) && CARGO_TARGET_DIR=$(RSBUILD) cargo build --offline --release -q
	cp $(RSBUILD)/release/bench $@

FORCE:
