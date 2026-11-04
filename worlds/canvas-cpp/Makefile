# The canvas World in C++ and its tests, over runtime/cpp and worlds/cli-cpp. Written by
# Claude on the claude-self-hosting branch; subject to change.
#
#   make test            the tests under g++ and clang++, then under ASan and UBSan with g++
#   make check           the tests once, with $(CXX) into build/$(CONFIG)
#   make lib             build/$(CONFIG)/libprism_world_canvas.a
#   make program PROG=DIR [ROOT=MainProgram]
#                        DIR/canvas: the C++ program prismc emitted into DIR, its root under
#                        this World (see README.md, "A compiled program")
#   make chess-smoke     chess compiled to C++ under Xvfb: click e2, then e4, exit 0 (Linux)
#
# The tests: test_canvas (no display), and on Linux xvfb_smoke (a window under xvfb-run,
# clicked through XTest), on macOS macos_smoke (a Cocoa window, pressed through Hooks),
# all run by `check`.

CXX ?= g++
CONFIG ?= default
SANITIZE ?=
OPT ?= -O1 -g
REPO := $(abspath ../..)
RT := $(REPO)/runtime/cpp
CLI := $(REPO)/worlds/cli-cpp
FONT := $(REPO)/worlds/canvas/fonts/DejaVuSans.ttf
BUILD := build/$(CONFIG)
# The window side: Xlib on Linux (src/app.cpp), Cocoa on macOS (src/app_macos.mm).
ifeq ($(shell uname -s),Darwin)
PKGS := cairo freetype2
APP_SRC := src/app_macos.mm
APP_FLAGS := -x objective-c++ -fobjc-arc
FRAMEWORKS := -framework Cocoa -framework QuartzCore
SMOKE := macos_smoke
# LeakSanitizer does not run on macOS on arm64.
LEAKS := 0
else
PKGS := cairo freetype2 x11 xext
APP_SRC := src/app.cpp
APP_FLAGS :=
FRAMEWORKS :=
SMOKE := xvfb_smoke
LEAKS := 1
endif
PKG_CFLAGS := $(shell pkg-config --cflags $(PKGS))
PKG_LIBS := $(shell pkg-config --libs $(PKGS)) $(FRAMEWORKS)
STD ?= -std=c++20
WARN ?= -Wall -Wextra -Werror
FLAGS := $(STD) $(WARN) $(OPT) $(SANITIZE) -Iinclude -I$(RT)/include -I$(CLI)/include $(PKG_CFLAGS)
RTLIB := $(RT)/build/$(CONFIG)/libprism_rtl.a
LIB := $(BUILD)/libprism_world_canvas.a
LIB_SRCS := src/world.cpp src/font.cpp src/render.cpp
LIB_OBJS := $(LIB_SRCS:src/%.cpp=$(BUILD)/%.o) $(BUILD)/app.o $(BUILD)/cli.o
HEADERS := $(wildcard include/prism_world_canvas/*.hpp)

.PHONY: test check lib program chess-smoke FORCE

test:
	$(MAKE) check CXX=g++ CONFIG=gcc
	$(MAKE) check CXX=clang++ CONFIG=clang
	$(MAKE) check CXX=g++ CONFIG=asan-gcc OPT="-O1 -g -fno-omit-frame-pointer" SANITIZE="-fsanitize=address,undefined -fno-sanitize-recover=undefined"

check: $(BUILD)/canvas_tests $(BUILD)/$(SMOKE)
	ASAN_OPTIONS=detect_leaks=$(LEAKS) $(BUILD)/canvas_tests
	$(BUILD)/$(SMOKE)

lib: $(LIB)

$(RTLIB): FORCE
	@$(MAKE) -s -C $(RT) lib CXX=$(CXX) CONFIG=$(CONFIG) OPT="$(OPT)" SANITIZE="$(SANITIZE)"

$(BUILD)/%.o: src/%.cpp $(HEADERS)
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -DPRISM_CANVAS_FONT='"$(FONT)"' -c $< -o $@

$(BUILD)/app.o: $(APP_SRC) $(HEADERS)
	@mkdir -p $(BUILD)
	$(CXX) $(APP_FLAGS) $(FLAGS) -c $< -o $@

$(BUILD)/cli.o: $(CLI)/src/cli.cpp $(CLI)/include/prism_world_cli/cli.hpp
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -c $< -o $@

$(LIB): $(LIB_OBJS)
	@rm -f $@
	ar rcs $@ $^

$(BUILD)/canvas_tests: tests/test_canvas.cpp $(RT)/tests/main.cpp $(LIB) $(RTLIB) $(HEADERS)
	$(CXX) $(FLAGS) -I$(RT)/tests -DPRISM_REPO='"$(REPO)"' -o $@ tests/test_canvas.cpp $(RT)/tests/main.cpp $(LIB) $(RTLIB) $(PKG_LIBS)

$(BUILD)/macos_smoke: tests/macos_smoke.cpp tests/smoke_program.hpp $(LIB) $(RTLIB) $(HEADERS)
	$(CXX) $(FLAGS) -o $@ tests/macos_smoke.cpp $(LIB) $(RTLIB) $(PKG_LIBS)

$(BUILD)/xvfb_smoke: tests/xvfb_smoke.cpp tests/robot.hpp tests/smoke_program.hpp $(LIB) $(RTLIB) $(HEADERS)
	$(CXX) $(FLAGS) -o $@ tests/xvfb_smoke.cpp $(LIB) $(RTLIB) $(PKG_LIBS) -lXtst -lpthread

$(BUILD)/robot: tests/robot.cpp tests/robot.hpp
	@mkdir -p $(BUILD)
	$(CXX) $(STD) $(WARN) $(OPT) $(PKG_CFLAGS) -o $@ tests/robot.cpp $(PKG_LIBS) -lXtst

# ---- A compiled program under this World ------------------------------------------------
#
# prismc's C++ program has its own main.cpp, which runs its root under the CLI or the Test
# World. Here that file is compiled with its `main` renamed, and src/program_main.cpp runs
# the root `crate::$(ROOT)` under this World instead. The flags are the emitted Makefile's.

PROG ?=
ROOT ?= MainProgram
PROG_BUILD := $(PROG)/build-canvas/$(CONFIG)
PROG_SRCS := $(filter-out $(PROG)/src/main.cpp,$(wildcard $(PROG)/src/*.cpp))
PROG_OBJS := $(PROG_SRCS:$(PROG)/src/%.cpp=$(PROG_BUILD)/%.o) $(PROG_BUILD)/emitted_main.o $(PROG_BUILD)/program_main.o
PROG_FLAGS := -std=gnu++20 $(OPT) $(SANITIZE) -I$(RT)/include -I$(CLI)/include -Iinclude $(PKG_CFLAGS) -I$(PROG)/src -Wall -Wno-unused -Wno-unused-parameter -Wno-unused-label

program: $(PROG)/canvas

$(PROG)/canvas: $(PROG_OBJS) $(LIB) $(RTLIB)
	$(CXX) $(PROG_FLAGS) -o $@ $(PROG_OBJS) $(LIB) $(RTLIB) $(PKG_LIBS)

$(PROG_BUILD)/emitted_main.o: $(PROG)/src/main.cpp
	@mkdir -p $(PROG_BUILD)
	$(CXX) $(PROG_FLAGS) -Dmain=prism_emitted_main -MMD -MP -c $< -o $@

$(PROG_BUILD)/program_main.o: src/program_main.cpp $(HEADERS)
	@mkdir -p $(PROG_BUILD)
	$(CXX) $(PROG_FLAGS) -DPRISM_ROOT=$(ROOT) -MMD -MP -c $< -o $@

$(PROG_BUILD)/%.o: $(PROG)/src/%.cpp
	@mkdir -p $(PROG_BUILD)
	$(CXX) $(PROG_FLAGS) -MMD -MP -c $< -o $@

-include $(wildcard $(PROG_BUILD)/*.d)

# ---- Chess under Xvfb ---------------------------------------------------------------------

chess-smoke: $(BUILD)/robot
	tests/chess_smoke.sh $(BUILD)/robot

FORCE:
