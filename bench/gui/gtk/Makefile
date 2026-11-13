# The GTK 3 table (table.c), for the benchmark.
#
#   make        target/bench/gtk/table, at -O2 with $(CC) (cc by default)
#
# Written by Claude on the claude-self-hosting branch; subject to change.

REPO := $(abspath ../../..)
OUT := $(REPO)/target/bench/gtk
CC ?= cc

$(OUT)/table: table.c
	@mkdir -p $(OUT)
	$(CC) -O2 -Wall -Wextra -o $@ table.c $(shell pkg-config --cflags --libs gtk+-3.0)
