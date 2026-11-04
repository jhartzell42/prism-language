# The CLI World in C++ and its tests, over runtime/cpp. Written by Claude, 4 November 2026,
# on the claude-self-hosting branch. Subject to change.
#
#   make test          the tests under g++ and clang++, then under ASan and UBSan with g++
#   make check         the tests once, with $(CXX) into build/$(CONFIG)

CXX ?= g++
# LeakSanitizer does not run on macOS on arm64.
LEAKS := $(if $(filter Darwin,$(shell uname -s)),0,1)
CONFIG ?= default
SANITIZE ?=
OPT ?= -O1 -g
RT := ../../runtime/cpp
BUILD := build/$(CONFIG)
FLAGS := -std=c++20 -Wall -Wextra -Werror $(OPT) $(SANITIZE) -Iinclude -I$(RT)/include -I$(RT)/tests
LIB := $(RT)/build/$(CONFIG)/libprism_rtl.a

.PHONY: test check FORCE

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

check: $(BUILD)/cli_tests
	ASAN_OPTIONS=detect_leaks=$(LEAKS) $(BUILD)/cli_tests

$(LIB): FORCE
	@$(MAKE) -s -C $(RT) lib CXX=$(CXX) CONFIG=$(CONFIG) OPT="$(OPT)" SANITIZE="$(SANITIZE)"

$(BUILD)/cli_tests: src/cli.cpp tests/test_cli.cpp include/prism_world_cli/cli.hpp $(RT)/tests/main.cpp $(LIB)
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -o $@ src/cli.cpp tests/test_cli.cpp $(RT)/tests/main.cpp $(LIB)

FORCE:
