# The Prism table under the canvas World, C++ and Rust, built for the benchmark (bench/gui/README.md).
#
#   make            both drivers
#   make cpp        target/bench/prism/driver: app.prism, with examples/chess's window
#                   library, compiled to C++ by the prism script's prismc, at -O3
#   make rust       target/bench/prism-rs/driver: the same compiled to Rust, cargo
#                   release (opt-level 3)
#   ./check.sh      both built, and each run once with `check` (bench/gui/README.md)
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
OPT ?= -O3
CONFIG := bench-$(notdir $(CXX))$(OPT)
RT := $(REPO)/runtime/cpp
CLI := $(REPO)/worlds/cli/cpp
CANVAS := $(REPO)/worlds/canvas/cpp
RTLIB := $(RT)/build/$(CONFIG)/libprism_rtl.a
CANVASLIB := $(CANVAS)/build/$(CONFIG)/libprism_world_canvas.a
ifeq ($(shell uname -s),Darwin)
SYS_LIBS := $(shell pkg-config --libs cairo freetype2) -framework Cocoa -framework QuartzCore
else
SYS_LIBS := $(shell pkg-config --libs cairo freetype2 x11 xext)
endif
# The C++ driver's allocator is jemalloc, as every emitted program's (runtime/cpp/README.md,
# pick AL): without it the C++ driver's link stops and says so (the Rust driver's does not
# need it).
ALLOC_LIBS := $(shell pkg-config --libs jemalloc 2>/dev/null)
ALLOC_READY = @pkg-config --exists jemalloc || { echo "the C++ driver links jemalloc, found with pkg-config, and pkg-config does not find it: apt install libjemalloc-dev pkg-config (Debian, Ubuntu), brew install jemalloc (macOS)"; false; }
FLAGS := -std=gnu++20 $(OPT) -I$(RT)/include -I$(CLI)/include -I$(CANVAS)/include $(shell pkg-config --cflags cairo freetype2) -I$(SRC) -Wall -Wno-unused -Wno-unused-parameter -Wno-unused-label
BUILD := $(OUT)/build-driver

.PHONY: all cpp rust emitted FORCE
all: cpp rust
cpp: $(OUT)/driver

# The program, emitted afresh whenever the sources or the installed prismc are newer than
# its main.cpp.
INSTALLED_PRISMC := $(wildcard $(REPO)/target/bootstrap/prismc)
$(SRC)/main.cpp: app.prism $(wildcard $(REPO)/examples/chess/window/*.prism) $(INSTALLED_PRISMC)
	cd $(REPO) && ./prism compile --target cpp bench/gui/prism examples/chess/window -o target/bench/prism

$(RTLIB): FORCE
	@$(MAKE) -s -C $(RT) lib CXX=$(CXX) CONFIG=$(CONFIG) OPT="$(OPT)"

$(CANVASLIB): FORCE | $(RTLIB)
	@$(MAKE) -s -C $(CANVAS) lib CXX=$(CXX) CONFIG=$(CONFIG) OPT="$(OPT)"

# Every emitted source but main.cpp, compiled as the emitted Makefile would.
emitted: $(SRC)/main.cpp
	@$(MAKE) -s -f $(abspath Makefile) objects

# the emitted sources are nested as the program's modules are (src/DIR/mod.cpp)
PROG_SRCS = $(filter-out $(SRC)/main.cpp,$(shell find $(SRC) -name '*.cpp' 2>/dev/null))
PROG_OBJS = $(PROG_SRCS:$(SRC)/%.cpp=$(BUILD)/%.o)

.PHONY: objects
objects: $(PROG_OBJS) $(BUILD)/emitted_main.o $(BUILD)/driver.o

$(BUILD)/%.o: $(SRC)/%.cpp
	@mkdir -p $(dir $@)
	$(CXX) $(FLAGS) -MMD -MP -c $< -o $@

$(BUILD)/emitted_main.o: $(SRC)/main.cpp
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -MMD -MP -Dmain=prism_emitted_main -c $< -o $@

$(BUILD)/driver.o: driver.cpp $(SRC)/programs.hpp
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -MMD -MP -c $< -o $@

-include $(shell find $(BUILD) -name '*.d' 2>/dev/null)

# The objects are globbed when the link runs: on a clean build none exist when make reads
# this file, before prismc has emitted their sources.
$(OUT)/driver: emitted $(CANVASLIB) $(RTLIB)
	$(ALLOC_READY)
	$(CXX) $(FLAGS) -o $@ $(shell find $(BUILD) -name '*.o') $(CANVASLIB) $(RTLIB) $(SYS_LIBS) $(ALLOC_LIBS)

# ---- Rust ---------------------------------------------------------------------------

RS := $(REPO)/target/bench/prism-rs
RSBUILD := $(REPO)/target/bench/prism-rs-build

rust: $(RS)/driver

$(RS)/src/main.rs: app.prism $(wildcard $(REPO)/examples/chess/window/*.prism) driver.rs $(INSTALLED_PRISMC)
	cd $(REPO) && ./prism compile bench/gui/prism examples/chess/window -o target/bench/prism-rs
	sed -i.emitted 's/^fn main() {$$/fn prism_emitted_main() {/' $@
	echo 'include!("$(abspath driver.rs)");' >> $@

$(RS)/driver: $(RS)/src/main.rs FORCE
	CARGO_TARGET_DIR=$(RSBUILD) cargo build --offline --release -q --manifest-path $(RS)/Cargo.toml
	cp $(RSBUILD)/release/prism $@

FORCE:
