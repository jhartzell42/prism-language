// The Prism table's driver for the Rust target, driver.cpp in Rust:
// bench.prism compiled to Rust runs under the Rust canvas World with `Hooks`, and each
// step of an operation's script is a press at a button's text, found in the scene the
// World last read. A timed step's clock starts when the press is queued and stops at the
// next `Hooks::settled`: every tick it caused has run and every window has presented.
//
//   driver OP RUNS WARMUP     prints `OP RUN MS` per timed run, then `peak_rss_bytes N`
//   driver OP RUNS WARMUP check
//                             also counts each step's work by phase, and fails the run
//                             when a scene draws outside the window
//
//
// The work is counted as driver.cpp counts it: instructions retired on macOS
// (`proc_pid_rusage`, `thread_selfcounts`) and on Linux through a perf_event counter per
// process and per thread; when the Linux kernel refuses one (ENOENT, EOPNOTSUPP, EACCES),
// each thread's CPU nanoseconds instead, and every `check` and `phases` line names its
// unit (`unit=instructions`, `unit=ns`). No libc crate: the few calls are declared here.
//
// bench/gui/prism/Makefile includes this file at the end of the emitted main.rs, whose own
// `main` it renames `prism_emitted_main`.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

mod bench_driver {
    use std::time::Instant;

    use prism_rtl::Pod;
    use prism_world_canvas_rtl::world::gui;
    use prism_world_canvas_rtl::{Hooks, Session, run_hooked};
    use prism_world_cli::UnwrapWorld;

    /// One press: the button whose text is `text`, or with `row` set, row `row`'s label
    /// (`text` empty) or its remove button (`text` "x"), counting from 1.
    #[derive(Clone)]
    struct Press {
        text: &'static str,
        row: usize,
        timed: bool,
    }

    const fn button(text: &'static str, timed: bool) -> Press {
        Press { text, row: 0, timed }
    }

    const CREATE_1K: &str = "Create 1,000 rows";
    const CREATE_10K: &str = "Create 10,000 rows";
    const APPEND_1K: &str = "Append 1,000 rows";
    const UPDATE: &str = "Update every 10th row";
    const CLEAR: &str = "Clear";
    const SWAP: &str = "Swap rows";

    /// Each operation's script for one run, as driver.cpp has them.
    fn script(op: &str) -> Option<Vec<Press>> {
        Some(match op {
            "create1k" => vec![button(CREATE_1K, true), button(CLEAR, false)],
            "replace1k" => vec![button(CREATE_1K, false), button(CREATE_1K, true), button(CLEAR, false)],
            "update10th" => vec![button(CREATE_1K, false), button(UPDATE, true), button(CLEAR, false)],
            "select" => vec![button(CREATE_1K, false), Press { text: "", row: 2, timed: true }, button(CLEAR, false)],
            "swap" => vec![button(CREATE_1K, false), button(SWAP, true), button(CLEAR, false)],
            "remove" => vec![button(CREATE_1K, false), Press { text: "x", row: 2, timed: true }, button(CLEAR, false)],
            "create10k" => vec![button(CREATE_10K, true), button(CLEAR, false)],
            "append1k" => vec![button(CREATE_1K, false), button(APPEND_1K, true), button(CLEAR, false)],
            "clear" => vec![button(CREATE_1K, false), button(CLEAR, true)],
            _ => return None,
        })
    }

    /// Every `Draw.Text` in a draw list, in drawing order, as (x, y, text).
    fn texts(draws: &Pod, out: &mut Vec<(f64, f64, String)>) {
        let Pod::Array(xs) = draws.structure() else { return };
        for d in xs {
            match d.structure().tag() {
                Some("Draw.Text") => {
                    let (Some(at), Some(t)) = (d.structure().field("at"), d.structure().field("text")) else { continue };
                    let Pod::Text(t) = t.structure() else { continue };
                    let n = |f: &str| at.field(f).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    out.push((n("x"), n("y"), t.to_string()));
                }
                Some("Draw.Clip") | Some("Draw.Group") => {
                    if let Some(inner) = d.structure().field("draws") {
                        texts(inner, out);
                    }
                }
                _ => {}
            }
        }
    }

    /// Where to press for `p`: just inside the text's point, which a button insets 8.
    fn target(s: &dyn Session, p: &Press) -> Option<(f64, f64)> {
        let mut all = Vec::new();
        for scene in s.shared().scenes.values() {
            texts(&scene.draw, &mut all);
        }
        let mut xs = 0;
        for (i, (x, y, t)) in all.iter().enumerate() {
            if p.row == 0 {
                if t == p.text {
                    return Some((x + 2.0, y + 2.0));
                }
                continue;
            }
            if t != "x" {
                continue;
            }
            xs += 1;
            if xs != p.row {
                continue;
            }
            if p.text == "x" {
                return Some((x + 2.0, y + 2.0));
            }
            let (lx, ly, _) = all.get(i.checked_sub(1)?)?;
            return Some((lx + 2.0, ly + 2.0));
        }
        None
    }

    /// The table the program should have, as driver.cpp's `Model` follows it: the ids its
    /// rows were made with, in slot order, the rows Update every 10th row marked, and the
    /// selected row's position counting from 1 (0 for none).
    #[derive(Default)]
    struct Model {
        next: u64,
        ids: Vec<u64>,
        banged: Vec<bool>,
        selected: usize,
    }

    impl Model {
        fn new() -> Model {
            Model { next: 1, ..Model::default() }
        }

        fn press(&mut self, p: &Press) {
            fn add(m: &mut Model, n: usize) {
                for _ in 0..n {
                    m.ids.push(m.next);
                    m.banged.push(false);
                    m.next += 1;
                }
            }
            if p.row != 0 && p.text == "x" {
                self.ids.remove(p.row - 1);
                self.banged.remove(p.row - 1);
                if self.selected == p.row {
                    self.selected = 0;
                } else if self.selected > p.row {
                    self.selected -= 1;
                }
            } else if p.row != 0 {
                self.selected = p.row;
            } else if p.text == CREATE_1K || p.text == CREATE_10K {
                // the new rows take new slots
                self.ids.clear();
                self.banged.clear();
                self.selected = 0;
                add(self, if p.text == CREATE_1K { 1000 } else { 10000 });
            } else if p.text == APPEND_1K {
                add(self, 1000);
            } else if p.text == CLEAR {
                self.ids.clear();
                self.banged.clear();
                self.selected = 0;
            } else if p.text == SWAP && self.ids.len() >= 999 {
                self.ids.swap(1, 998);
                self.banged.swap(1, 998);
            } else if p.text == UPDATE {
                for i in (0..self.banged.len()).step_by(10) {
                    self.banged[i] = true;
                }
            }
        }
    }

    /// The rows the screen shows, top to bottom, as (id, label, the label's y): a row's
    /// texts are its id, its label (the select button's) and its remove button's "x".
    fn rows_shown(s: &dyn Session) -> Vec<(String, String, f64)> {
        let mut all = Vec::new();
        for scene in s.shared().scenes.values() {
            texts(&scene.draw, &mut all);
        }
        let mut rows = Vec::new();
        for i in 2..all.len() {
            if all[i].2 == "x" {
                rows.push((all[i - 2].2.clone(), all[i - 1].2.clone(), all[i - 1].1));
            }
        }
        rows
    }

    /// Whether the screen shows the table the model says, as driver.cpp's `shows` checks
    /// it; says what it saw when not.
    fn shows(s: &dyn Session, m: &Model) -> bool {
        let rows = rows_shown(s);
        let h = s.shared().windows.values().next().map_or(0.0, |w| w.size.1);
        let fail = |why: String| {
            eprintln!("driver: the screen shows {} rows of {}: {why}", rows.len(), m.ids.len());
            false
        };
        if rows.len() > m.ids.len() {
            return fail("more than the table has".into());
        }
        if !m.ids.is_empty() && rows.is_empty() {
            return fail("none".into());
        }
        if rows.len() < m.ids.len() && rows.last().is_some_and(|r| r.2 < h - 60.0) {
            return fail("they stop short of the window's bottom edge".into());
        }
        for (i, (id, label, _)) in rows.iter().enumerate() {
            let n = i + 1;
            if *id != m.ids[i].to_string() {
                return fail(format!("row {n} is id {id}, not {}", m.ids[i]));
            }
            let sel = label.starts_with("\u{25b6} ");
            if sel != (m.selected == n) {
                return fail(format!("row {n} {} marked selected", if sel { "is" } else { "is not" }));
            }
            if label.ends_with(" !!!") != m.banged[i] {
                return fail(format!("row {n}'s label is {label:?}"));
            }
        }
        true
    }

    /// The whole process's work so far, every thread's: instructions retired (macOS's
    /// `proc_pid_rusage`, as driver.cpp's `process_work`).
    #[cfg(target_os = "macos")]
    fn process_work() -> u64 {
        // `struct rusage_info_v4` (sys/resource.h) is 296 bytes, its `ri_instructions` the
        // 32nd 8-byte word (byte 248, after a 16-byte uuid and 29 counters); the buffer is
        // that size exactly, 8-aligned.
        let mut u = [0u64; 37];
        let r = unsafe { proc_pid_rusage(std::process::id() as i32, 4, u.as_mut_ptr().cast()) };
        if r != 0 {
            eprintln!("driver: proc_pid_rusage failed");
            std::process::exit(1);
        }
        u[31]
    }

    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut std::ffi::c_void) -> i32;
        /// libsystem_kernel's `thread_selfcounts` (exported, in no header): flavour 1
        /// fills the calling thread's instructions and cycles retired, two `u64`s.
        fn thread_selfcounts(flavor: i32, buffer: *mut u64, bytes: usize) -> i32;
    }

    /// Instructions the calling thread has retired so far.
    #[cfg(target_os = "macos")]
    fn thread_work() -> u64 {
        let mut c = [0u64; 2];
        // Safety: `c` is two writable, aligned `u64`s and the call is told so in bytes;
        // flavour 1 writes exactly that much (instructions, then cycles) and keeps no
        // pointer to it.
        let r = unsafe { thread_selfcounts(1, c.as_mut_ptr(), std::mem::size_of_val(&c)) };
        if r != 0 {
            eprintln!("driver: thread_selfcounts failed");
            std::process::exit(1);
        }
        c[0]
    }

    #[cfg(target_os = "macos")]
    mod work {
        /// macOS counts instructions; nothing to choose.
        pub fn choose() {}
        pub fn name() -> &'static str {
            "instructions"
        }
    }

    /// What a step's work is counted in on Linux: a perf_event instructions counter
    /// (PERF_COUNT_HW_INSTRUCTIONS) when the kernel opens one, all instructions if allowed
    /// and user-space ones under perf_event_paranoid 2, else CPU nanoseconds.
    #[cfg(target_os = "linux")]
    mod work {
        use std::cell::Cell;
        use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};

        pub const INSTRUCTIONS: u8 = 0;
        pub const INSTRUCTIONS_USER: u8 = 1;
        pub const NS: u8 = 2;
        static UNIT: AtomicU8 = AtomicU8::new(NS);
        /// The process-wide counter, inherited by the threads made after it.
        static PROCESS_FD: AtomicI32 = AtomicI32::new(-1);

        /// `struct perf_event_attr`'s first 128 bytes (PERF_ATTR_SIZE_VER7): the kernel
        /// takes any size from 64 up with the unused tail zero.
        #[repr(C)]
        struct PerfEventAttr {
            type_: u32,
            size: u32,
            config: u64,
            sample_period: u64,
            sample_type: u64,
            read_format: u64,
            flags: u64,
            rest: [u64; 10],
        }
        const PERF_TYPE_HARDWARE: u32 = 0;
        const PERF_COUNT_HW_INSTRUCTIONS: u64 = 1;
        const PERF_FLAG_FD_CLOEXEC: u64 = 8;
        const FLAG_INHERIT: u64 = 1 << 1;
        const FLAG_EXCLUDE_KERNEL: u64 = 1 << 5;
        const FLAG_EXCLUDE_HV: u64 = 1 << 6;
        #[cfg(target_arch = "x86_64")]
        const SYS_PERF_EVENT_OPEN: i64 = 298;
        #[cfg(target_arch = "aarch64")]
        const SYS_PERF_EVENT_OPEN: i64 = 241;
        pub const CLOCK_PROCESS_CPUTIME_ID: i32 = 2;
        pub const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
        const EPERM: i32 = 1;
        const ENOENT: i32 = 2;
        const EACCES: i32 = 13;
        const ENOSYS: i32 = 38;
        const EOPNOTSUPP: i32 = 95;

        #[repr(C)]
        struct Timespec {
            tv_sec: i64,
            tv_nsec: i64,
        }

        unsafe extern "C" {
            fn syscall(num: i64, ...) -> i64;
            fn read(fd: i32, buf: *mut std::ffi::c_void, count: usize) -> isize;
            fn clock_gettime(clock: i32, t: *mut Timespec) -> i32;
        }

        /// A counter on the calling thread, `inherit` counting the threads it makes
        /// afterwards too; the fd, or the errno.
        fn perf_open(inherit: bool, user_only: bool) -> Result<i32, i32> {
            let mut flags = 0;
            if inherit {
                flags |= FLAG_INHERIT;
            }
            if user_only {
                flags |= FLAG_EXCLUDE_KERNEL | FLAG_EXCLUDE_HV;
            }
            let attr = PerfEventAttr {
                type_: PERF_TYPE_HARDWARE,
                size: std::mem::size_of::<PerfEventAttr>() as u32,
                config: PERF_COUNT_HW_INSTRUCTIONS,
                sample_period: 0,
                sample_type: 0,
                read_format: 0,
                flags,
                rest: [0; 10],
            };
            // Safety: perf_event_open(attr, pid 0 = this thread, cpu -1 = any, no group,
            // flags); the kernel reads `attr.size` bytes of `attr`, which is that size.
            let fd = unsafe { syscall(SYS_PERF_EVENT_OPEN, &attr as *const PerfEventAttr, 0i32, -1i32, -1i32, PERF_FLAG_FD_CLOEXEC) };
            if fd >= 0 { Ok(fd as i32) } else { Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0)) }
        }

        fn refused(err: i32) -> bool {
            matches!(err, ENOENT | EOPNOTSUPP | EACCES | EPERM | ENOSYS)
        }

        /// Chooses the unit, once, before any other thread exists: the process counter
        /// opened with inherit, all instructions if allowed, else user-space ones, else
        /// nanoseconds. Says on stderr when it is not all instructions.
        pub fn choose() {
            for user_only in [false, true] {
                match perf_open(true, user_only) {
                    Ok(fd) => {
                        PROCESS_FD.store(fd, Ordering::Relaxed);
                        UNIT.store(if user_only { INSTRUCTIONS_USER } else { INSTRUCTIONS }, Ordering::Relaxed);
                        if user_only {
                            eprintln!("driver: counting user-space instructions only (the kernel refused kernel counting: perf_event_paranoid)");
                        }
                        return;
                    }
                    Err(e) if refused(e) => {
                        if user_only {
                            eprintln!("driver: no instructions counter on this machine (perf_event_open: {}); counting CPU nanoseconds instead", std::io::Error::from_raw_os_error(e));
                            UNIT.store(NS, Ordering::Relaxed);
                        }
                    }
                    Err(e) => {
                        eprintln!("driver: perf_event_open: {}", std::io::Error::from_raw_os_error(e));
                        std::process::exit(1);
                    }
                }
            }
        }

        pub fn unit() -> u8 {
            UNIT.load(Ordering::Relaxed)
        }

        pub fn name() -> &'static str {
            match unit() {
                INSTRUCTIONS => "instructions",
                INSTRUCTIONS_USER => "instructions-user",
                _ => "ns",
            }
        }

        pub fn read_counter(fd: i32) -> u64 {
            let mut v = 0u64;
            // Safety: a perf counter's read fills one u64, and `v` is one.
            let n = unsafe { read(fd, (&mut v as *mut u64).cast(), 8) };
            if n != 8 {
                eprintln!("driver: reading the perf_event counter: {}", std::io::Error::last_os_error());
                std::process::exit(1);
            }
            v
        }

        pub fn cpu_ns(clock: i32) -> u64 {
            let mut t = Timespec { tv_sec: 0, tv_nsec: 0 };
            // Safety: `t` is a writable timespec of the C layout.
            if unsafe { clock_gettime(clock, &mut t) } != 0 {
                eprintln!("driver: clock_gettime: {}", std::io::Error::last_os_error());
                std::process::exit(1);
            }
            t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
        }

        pub fn process() -> u64 {
            if unit() == NS {
                return cpu_ns(CLOCK_PROCESS_CPUTIME_ID);
            }
            read_counter(PROCESS_FD.load(Ordering::Relaxed))
        }

        thread_local! {
            static FD: Cell<i32> = const { Cell::new(-1) };
        }

        /// This thread's own counter (inherit off), opened at its first call.
        pub fn thread() -> u64 {
            if unit() == NS {
                return cpu_ns(CLOCK_THREAD_CPUTIME_ID);
            }
            let fd = FD.get();
            let fd = if fd >= 0 {
                fd
            } else {
                match perf_open(false, unit() == INSTRUCTIONS_USER) {
                    Ok(fd) => {
                        FD.set(fd);
                        fd
                    }
                    Err(e) => {
                        eprintln!("driver: perf_event_open for a thread (the process's counter opened): {}", std::io::Error::from_raw_os_error(e));
                        std::process::exit(1);
                    }
                }
            };
            read_counter(fd)
        }
    }

    /// The whole process's work so far, every thread's, in `work::name()`'s unit.
    #[cfg(target_os = "linux")]
    fn process_work() -> u64 {
        work::process()
    }

    /// The calling thread's work so far, in `work::name()`'s unit.
    #[cfg(target_os = "linux")]
    fn thread_work() -> u64 {
        work::thread()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn process_work() -> u64 {
        eprintln!("driver: check counts work with macOS's proc_pid_rusage or Linux's perf_event, which this system lacks");
        std::process::exit(2);
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn thread_work() -> u64 {
        process_work()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    mod work {
        pub fn choose() {}
        pub fn name() -> &'static str {
            "unknown"
        }
    }

    /// The work of each phase (prism_rtl's `world::probe`), by what each thread did
    /// between its marks (`work::name()`'s unit): each thread's current phase and its
    /// count at its last mark, and the totals per phase over every thread.
    mod phases {
        use std::cell::Cell;
        use std::sync::atomic::{AtomicU64, Ordering};

        use prism_rtl::world::probe::{PHASES, Phase};

        pub static TOTAL: [AtomicU64; PHASES] = [const { AtomicU64::new(0) }; PHASES];
        pub static TICKS: AtomicU64 = AtomicU64::new(0);

        thread_local! {
            static AT: Cell<Option<(Phase, u64)>> = const { Cell::new(None) };
        }

        /// The probe's hook: this thread's work since its last mark goes to the phase it
        /// was in.
        pub fn mark(next: Phase) {
            let now = super::thread_work();
            if let Some((was, since)) = AT.get() {
                TOTAL[was as usize].fetch_add(now - since, Ordering::Relaxed);
            }
            if next == Phase::Propagate {
                TICKS.fetch_add(1, Ordering::Relaxed);
            }
            AT.set(Some((next, now)));
        }

        /// This thread's work since its last mark, counted to its phase.
        pub fn flush() {
            if let Some((was, since)) = AT.get() {
                let now = super::thread_work();
                TOTAL[was as usize].fetch_add(now - since, Ordering::Relaxed);
                AT.set(Some((was, now)));
            }
        }

        /// The totals so far, and the ticks.
        pub fn now() -> ([u64; PHASES], u64) {
            flush();
            (std::array::from_fn(|i| TOTAL[i].load(Ordering::Relaxed)), TICKS.load(Ordering::Relaxed))
        }

        /// One step's line: each phase's work from `from` to `to`, the rest of the
        /// process's (`all`, every thread's) as `other`, the ticks, and the unit.
        pub fn line(op: &str, kind: &str, from: &([u64; PHASES], u64), to: &([u64; PHASES], u64), all: u64) -> String {
            let names = ["loop", "propagate", "build", "patch", "diff", "perform", "compact", "paint"];
            let mut out = format!("phases {op} {kind}");
            let mut sum = 0;
            for i in 0..PHASES {
                let d = to.0[i] - from.0[i];
                sum += d;
                out += &format!(" {}={d}", names[i]);
            }
            out += &format!(" other={} total={all} ticks={} unit={}", all.saturating_sub(sum), to.1 - from.1, super::work::name());
            out
        }
    }

    /// `check`'s first check, as driver.cpp's `drawn_inside`.
    fn drawn_inside(s: &dyn Session) -> bool {
        let Some((ww, wh)) = s.shared().windows.values().next().map(|w| w.size) else { return true };
        let mut all = Vec::new();
        for scene in s.shared().scenes.values() {
            texts(&scene.draw, &mut all);
        }
        let outside = all.iter().filter(|(x, y, _)| *x > ww + 8.0 || *y > wh + 8.0 || *x < -8.0 || *y < -8.0).count();
        if outside == 0 {
            return true;
        }
        eprintln!("driver: check: {outside} of the scenes' {} texts are drawn outside the {ww}x{wh} window", all.len());
        false
    }

    fn peak_rss_bytes() -> i64 {
        let mut u: libc_rusage = unsafe { std::mem::zeroed() };
        unsafe { getrusage(0, &mut u) };
        if cfg!(target_os = "macos") { u.ru_maxrss } else { u.ru_maxrss * 1024 }
    }

    /// `struct rusage` as far as `ru_maxrss`: two `timeval`s, then the field.
    #[repr(C)]
    struct libc_rusage {
        ru_utime: [i64; 2],
        ru_stime: [i64; 2],
        ru_maxrss: i64,
        rest: [i64; 13],
    }

    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut libc_rusage) -> i32;
    }

    pub fn main() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let (op, runs, warmup, check) = match args.as_slice() {
            [op, r, w] => (op.clone(), r.parse::<usize>().unwrap_or(0), w.parse::<usize>().ok(), false),
            [op, r, w, c] if c == "check" => (op.clone(), r.parse::<usize>().unwrap_or(0), w.parse::<usize>().ok(), true),
            _ => (String::new(), 0, None, false),
        };
        let (Some(one), Some(warmup)) = (script(&op), warmup) else {
            eprintln!("usage: driver OP RUNS WARMUP [check] (OP: create1k replace1k update10th select swap remove create10k append1k clear)");
            std::process::exit(2);
        };
        if runs == 0 {
            eprintln!("driver: RUNS must be at least 1");
            std::process::exit(2);
        }
        let mut steps = Vec::new();
        for r in 0..warmup + runs {
            for p in &one {
                let mut p = p.clone();
                p.timed &= r >= warmup;
                steps.push(p);
            }
        }
        let mut next = 0;
        let mut run = 0;
        let mut waiting = false;
        let mut timing = false;
        let mut t0 = Instant::now();
        let mut i0 = 0u64;
        let mut p0 = ([0u64; prism_rtl::world::probe::PHASES], 0u64);
        if check {
            work::choose(); // before the World's threads exist, so the process counter inherits to them
        }
        if check && !prism_rtl::world::probe::set_hook(phases::mark) {
            eprintln!("driver: the phase probe's hook was set already");
            std::process::exit(1);
        }
        let mut model = Model::new();
        let failed = std::rc::Rc::new(std::cell::Cell::new(false));
        let failed_in = failed.clone();
        let hooks = Hooks {
            settled: Some(Box::new(move |s: &mut dyn Session| {
                if waiting {
                    let work = if check { process_work() - i0 } else { 0 };
                    let p1 = if check { phases::now() } else { p0 };
                    if timing {
                        run += 1;
                        println!("{op} {run} {:.3}", t0.elapsed().as_secs_f64() * 1e3);
                        if !shows(s, &model) {
                            failed_in.set(true);
                            s.exit(1);
                            return;
                        }
                    }
                    if check {
                        let p = &steps[next - 1];
                        println!("check {op} step {next} ({} row {}): {work} {}", p.text, p.row, work::name());
                        let kind = if timing { "timed" } else if p.text == CREATE_1K { "setup" } else { "untimed" };
                        println!("{}", phases::line(&op, kind, &p0, &p1, work));
                        if !drawn_inside(s) {
                            failed_in.set(true);
                            s.exit(1);
                            return;
                        }
                    }
                }
                waiting = false;
                if next == steps.len() {
                    println!("peak_rss_bytes {}", peak_rss_bytes());
                    s.exit(0);
                    return;
                }
                let Some(window) = s.shared().windows.keys().next().cloned() else { return };
                let p = steps[next].clone();
                next += 1;
                let Some((x, y)) = target(s, &p) else {
                    eprintln!("driver: no button \"{}\" (row {}) on the screen", p.text, p.row);
                    failed_in.set(true);
                    s.exit(1);
                    return;
                };
                waiting = true;
                timing = p.timed;
                model.press(&p);
                if check {
                    p0 = phases::now();
                    i0 = process_work();
                }
                t0 = Instant::now();
                s.input(&window, gui::press(x, y));
            })),
        };
        let code = run_hooked::<crate::MainProgram>(|w| Box::new(UnwrapWorld::new(w)), hooks);
        std::process::exit(if failed.get() { 1 } else { code as i32 });
    }
}

fn main() {
    bench_driver::main()
}
