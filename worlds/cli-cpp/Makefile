# The CLI World in C++ and its tests, over runtime/cpp. Written by Claude, 4 November 2026,
# on the claude-self-hosting branch. Subject to change.
#
#   make test          the tests under g++ and clang++, then under ASan and UBSan with g++
#   make check         the tests once, with $(CXX) into build/$(CONFIG)

CXX ?= g++
CONFIG ?= default
SANITIZE ?=
OPT ?= -O1 -g
RT := ../../runtime/cpp
BUILD := build/$(CONFIG)
FLAGS := -std=c++20 -Wall -Wextra -Werror $(OPT) $(SANITIZE) -Iinclude -I$(RT)/include -I$(RT)/tests
LIB := $(RT)/build/$(CONFIG)/libprism_rtl.a

.PHONY: test check FORCE

test:
	$(MAKE) check CXX=g++ CONFIG=gcc
	$(MAKE) check CXX=clang++ CONFIG=clang
	$(MAKE) check CXX=g++ CONFIG=asan-gcc OPT="-O1 -g -fno-omit-frame-pointer" SANITIZE="-fsanitize=address,undefined -fno-sanitize-recover=undefined"

check: $(BUILD)/cli_tests
	ASAN_OPTIONS=detect_leaks=1 $(BUILD)/cli_tests

$(LIB): FORCE
	@$(MAKE) -s -C $(RT) lib CXX=$(CXX) CONFIG=$(CONFIG) OPT="$(OPT)" SANITIZE="$(SANITIZE)"

$(BUILD)/cli_tests: src/cli.cpp tests/test_cli.cpp include/prism_world_cli/cli.hpp $(RT)/tests/main.cpp $(LIB)
	@mkdir -p $(BUILD)
	$(CXX) $(FLAGS) -o $@ src/cli.cpp tests/test_cli.cpp $(RT)/tests/main.cpp $(LIB)

FORCE:
