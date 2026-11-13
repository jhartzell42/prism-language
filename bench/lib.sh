# The benchmark scripts' shared measuring:
# sourced by bench/building/run.sh, bench/running/run.sh, bench/gui/run.sh and
# tests/cost/run.sh. Each platform has the same facts under different names, and this file
# is where the names are chosen, so the scripts read alike.
#
#   bench_os                 Darwin or Linux (`uname -s`)
#   machine_line             the processor, cores, memory and OS, one line
#   load_line                the load average
#   time_cmd                 the words that run a command under the system's `time` with
#                            its resource report: `/usr/bin/time -l` (macOS), `-v` (GNU)
#   peak_mb_of LOG           the peak resident set, MB, from that report
#   instr_setup              works out how instructions are counted here, into
#                            $instr_counter: rusage (macOS's `time -l` reports "instructions
#                            retired"), perf (`perf stat` with hardware counters), or none,
#                            with $instr_why saying why not
#   instr_prefix OUT         the words before a command so that its instructions land in
#                            OUT: nothing on macOS (time_cmd's report has them), `perf
#                            stat ... -o OUT --` under perf; nothing under none
#   instr_of TIMELOG OUT     the count those left, or "n/a" under none
#   instructions_of CMD...   CMD's instructions, counted exactly: `time -l` on macOS,
#                            `perf stat` with counters, else valgrind's cachegrind ("I
#                            refs", deterministic, about fifty times slower); for the
#                            small runs of tests/cost; instr_tool names what counts.
#
# Every missing tool is a hard error that names it. Nothing here is skipped silently.
#
# Written by Claude on the claude-self-hosting branch; subject to change.

bench_os=$(uname -s)

bench_die() { echo "bench: $*" >&2; exit 2; }

machine_line() {
    local cores; cores=$(getconf _NPROCESSORS_ONLN)
    case "$bench_os" in
        Darwin)
            echo "$(sysctl -n machdep.cpu.brand_string), $cores cores, $(( $(sysctl -n hw.memsize) / 1073741824 )) GB, $(sw_vers -productName) $(sw_vers -productVersion)" ;;
        Linux)
            local cpu mem os
            cpu=$(awk -F': *' '/^model name/ { print $2; exit }' /proc/cpuinfo)
            mem=$(awk '/^MemTotal:/ { printf "%.0f", $2 / 1048576 }' /proc/meminfo)
            os=$(. /etc/os-release && echo "$PRETTY_NAME")
            echo "${cpu:-unknown processor}, $cores cores, $mem GB, $os, Linux $(uname -r)" ;;
        *) bench_die "unknown platform $bench_os (Darwin or Linux)" ;;
    esac
}

load_line() {
    case "$bench_os" in
        Darwin) sysctl -n vm.loadavg ;;
        *) cut -d' ' -f1-3 /proc/loadavg ;;
    esac
}

# The system's `time` with its resource report: BSD's `-l` on macOS, GNU's `-v` elsewhere
# (the shell's keyword `time` reports no memory, so it must be the binary).
time_cmd() {
    [ -x /usr/bin/time ] || bench_die "no /usr/bin/time (install GNU time: apt install time)"
    case "$bench_os" in
        Darwin) echo "/usr/bin/time -l" ;;
        *) /usr/bin/time --version 2>&1 | grep -q 'GNU' || bench_die "/usr/bin/time is not GNU time (apt install time)"
           echo "/usr/bin/time -v" ;;
    esac
}

# The peak resident set, MB, from time_cmd's report: macOS prints bytes ("maximum resident
# set size"), GNU time kilobytes ("Maximum resident set size (kbytes)").
peak_mb_of() {
    awk '/maximum resident set size/ { printf "%.0f", $1 / 1048576; exit }
         /Maximum resident set size \(kbytes\)/ { printf "%.0f", $NF / 1024; exit }' "$1"
}

# perf's binary: `perf` when it runs (Ubuntu's /usr/bin/perf is a wrapper that refuses when
# no linux-tools matches the running kernel), else the newest linux-tools' own.
perf_bin() {
    if command -v perf > /dev/null && perf version > /dev/null 2>&1; then echo perf; return; fi
    local p
    p=$(ls -d /usr/lib/linux-tools*/perf 2>/dev/null | sort -V | tail -1)
    [ -n "$p" ] && [ -x "$p" ] && echo "$p"
}

# Sets $instr_counter to rusage, perf or none (and $instr_why to why none, $perf_cmd to
# perf's path), once; the scripts call it at their start, so the choice is made before any
# measuring and reported with the numbers.
instr_setup() {
    [ -z "${instr_counter:-}" ] || return 0
    instr_why=""
    case "$bench_os" in
        Darwin) instr_counter=rusage ;;
        *)
            local perf out
            perf=$(perf_bin)
            if [ -z "$perf" ]; then
                instr_counter=none; instr_why="perf is not installed"
            else
                out=$("$perf" stat -e instructions -x, -- true 2>&1 || true)
                case "$out" in
                    *'<not supported>'*) instr_counter=none; instr_why="perf ($perf) reports the instructions counter <not supported>: this machine exposes no hardware counters" ;;
                    *,,instructions,*) instr_counter=perf; perf_cmd=$perf ;;
                    *) instr_counter=none; instr_why="perf ($perf) cannot count instructions: $(echo "$out" | head -1)" ;;
                esac
            fi ;;
    esac
}

# The words before a command whose instructions go to OUT (perf's CSV report), or nothing.
instr_prefix() {
    instr_setup
    case "$instr_counter" in
        perf) echo "$perf_cmd stat -e instructions -x, -o $1 --" ;;
        *) echo "" ;;
    esac
}

# The count time_cmd's report (TIMELOG) or perf's (OUT) holds, or "n/a".
instr_of() {
    instr_setup
    case "$instr_counter" in
        rusage) awk '/instructions retired/ { print $1; exit }' "$1" ;;
        perf) awk -F, '$3 == "instructions" { print $1; exit }' "$2" ;;
        *) echo "n/a" ;;
    esac
}

# What instructions_of counts with, named for the output: macOS's `time -l`, perf, or
# valgrind's cachegrind when the machine has no counters (the reason beside it).
instr_tool() {
    instr_setup
    case "$instr_counter" in
        rusage) echo "macOS time -l (instructions retired)" ;;
        perf) echo "perf stat (instructions)" ;;
        *) echo "valgrind cachegrind (I refs; $instr_why)" ;;
    esac
}

# A command's instructions, exactly: `time -l` on macOS, `perf stat` with counters, else
# valgrind's cachegrind (its "I refs": every instruction the program executes, the same
# count on every run; slow). The command's stdout is dropped.
instructions_of() {
    local n
    instr_setup
    case "$instr_counter" in
        rusage)
            n=$(/usr/bin/time -l "$@" 2>&1 > /dev/null | awk '/instructions retired/ { print $1; exit }') ;;
        perf)
            n=$("$perf_cmd" stat -e instructions -x, -- "$@" 2>&1 > /dev/null | awk -F, '$3 == "instructions" { print $1; exit }') ;;
        *)
            command -v valgrind > /dev/null || bench_die "no hardware counters here ($instr_why) and no valgrind to count instructions with (apt install valgrind)"
            n=$(valgrind --tool=cachegrind --cache-sim=no --cachegrind-out-file=/dev/null "$@" 2>&1 > /dev/null | awk '/I +refs:/ { gsub(",", "", $NF); print $NF; exit }') ;;
    esac
    [ -n "$n" ] || bench_die "$(instr_tool) gave no instruction count for: $*"
    echo "$n"
}
