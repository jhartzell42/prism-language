# prism_rtl in C++: a static library and its tests. Written by Claude, 3 November 2026,
# on the claude-self-hosting branch. Subject to change.
#
#   make test          the tests under g++ and clang++, then both under
#                      AddressSanitizer, LeakSanitizer and UBSan
#   make check         the tests once, with $(CXX) into build/$(CONFIG)
#   make lib           build/$(CONFIG)/libprism_rtl.a

CXX ?= g++
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

test:
	$(MAKE) check CXX=g++ CONFIG=gcc
	$(MAKE) check CXX=clang++ CONFIG=clang
	$(MAKE) check CXX=g++ CONFIG=asan-gcc OPT="-O1 -g -fno-omit-frame-pointer" SANITIZE="-fsanitize=address,undefined -fno-sanitize-recover=undefined"
	$(MAKE) check CXX=clang++ CONFIG=asan-clang OPT="-O1 -g -fno-omit-frame-pointer" SANITIZE="-fsanitize=address,undefined -fno-sanitize-recover=undefined"

check: $(BUILD)/prism_rtl_tests
	ASAN_OPTIONS=detect_leaks=1 $(BUILD)/prism_rtl_tests

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
