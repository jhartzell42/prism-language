# prism_rtl in C++: a static library and its tests. Written by Claude, 3 November 2026,
# on the claude-self-hosting branch. Subject to change.
#
#   make test          the tests under g++ and clang++, then under AddressSanitizer,
#                      LeakSanitizer and UBSan with g++ (and clang++ when its
#                      sanitizer runtime is installed)
#   make check         the tests once, with $(CXX) into build/$(CONFIG)
#   make lib           build/$(CONFIG)/libprism_rtl.a

CXX ?= g++
# LeakSanitizer does not run on macOS on arm64.
LEAKS := $(if $(filter Darwin,$(shell uname -s)),0,1)
CONFIG ?= default
SANITIZE ?=
STD ?= -std=c++20
WARN := -Wall -Wextra -Werror
OPT ?= -O1 -g
TESTS_DIR := $(abspath ../../tests)

BUILD := build/$(CONFIG)
FLAGS := $(STD) $(WARN) $(OPT) $(SANITIZE) -Iinclude
SRCS := $(wildcard src/*.cpp)
TEST_SRCS := $(wildcard tests/*.cpp)
OBJS := $(SRCS:src/%.cpp=$(BUILD)/src/%.o)
TEST_OBJS := $(TEST_SRCS:tests/%.cpp=$(BUILD)/tests/%.o)
HEADERS := $(wildcard include/prism_rtl/*.hpp) $(wildcard tests/*.hpp)

.PHONY: test check lib clean

# The sanitizer run's compiler: g++ on Linux; on macOS Homebrew's clang (`brew install
# llvm`), since Apple clang 17's AddressSanitizer hangs at startup on macOS 26 (a program
# of an empty `main` never gets past its shadow-memory setup). Without it the run stops
# and says so.
ifeq ($(shell uname -s),Darwin)
ASAN_CXX := /opt/homebrew/opt/llvm/bin/clang++
ASAN_CONFIG := asan-llvm
# its own libc++, whose headers it compiles against (the system's lacks parts of them)
ASAN_LIBCXX := -L/opt/homebrew/opt/llvm/lib/c++ -Wl,-rpath,/opt/homebrew/opt/llvm/lib/c++ -Wno-unused-command-line-argument
ASAN_READY = @test -x $(ASAN_CXX) || { echo "the sanitizer run needs Homebrew's clang (brew install llvm): Apple clang's AddressSanitizer hangs at startup on this macOS"; false; }
else
ASAN_CXX := g++
ASAN_CONFIG := asan-gcc
ASAN_LIBCXX :=
ASAN_READY = @true
endif

test:
	$(MAKE) check CXX=g++ CONFIG=gcc
	$(MAKE) check CXX=clang++ CONFIG=clang
	$(ASAN_READY)
	$(MAKE) check CXX=$(ASAN_CXX) CONFIG=$(ASAN_CONFIG) OPT="-O1 -g -fno-omit-frame-pointer" SANITIZE="-fsanitize=address,undefined -fno-sanitize-recover=undefined $(ASAN_LIBCXX)"
	@if [ -f "$$(clang++ -print-resource-dir)/lib/linux/libclang_rt.asan-x86_64.a" ]; then \
	  $(MAKE) check CXX=clang++ CONFIG=asan-clang OPT="-O1 -g -fno-omit-frame-pointer" SANITIZE="-fsanitize=address,undefined -fno-sanitize-recover=undefined"; \
	else echo "clang++ has no sanitizer runtime here (libclang-rt-dev): the sanitizers ran under g++ only"; fi

check: $(BUILD)/prism_rtl_tests
	ASAN_OPTIONS=detect_leaks=$(LEAKS) $(BUILD)/prism_rtl_tests

lib: $(BUILD)/libprism_rtl.a

$(BUILD)/libprism_rtl.a: $(OBJS)
	ar rcs $@ $^

$(BUILD)/prism_rtl_tests: $(TEST_OBJS) $(BUILD)/libprism_rtl.a
	$(CXX) $(FLAGS) -o $@ $(TEST_OBJS) $(BUILD)/libprism_rtl.a

$(BUILD)/src/%.o: src/%.cpp $(HEADERS)
	@mkdir -p $(dir $@)
	$(CXX) $(FLAGS) -c $< -o $@

$(BUILD)/tests/%.o: tests/%.cpp $(HEADERS)
	@mkdir -p $(dir $@)
	$(CXX) $(FLAGS) -DPRISM_TESTS_DIR='"$(TESTS_DIR)"' -c $< -o $@

clean:
	rm -rf build
