// The Prism table's driver for the Rust target (bench/gui/README.md), driver.cpp in Rust:
// bench.prism compiled to Rust runs under the Rust canvas World with `Hooks`, and each
// step of an operation's script is a press at a button's text, found in the scene the
// World last read. A timed step's clock starts when the press is queued and stops at the
// next `Hooks::settled`: every tick it caused has run and every window has presented.
//
//   driver OP RUNS WARMUP     prints `OP RUN MS` per timed run, then `peak_rss_bytes N`
//   driver OP RUNS WARMUP check
//                             also counts each step's work and fails the run when it is
//                             out of proportion (bench/gui/README.md, "The checks")
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

    /// Instructions retired by the whole process so far, every thread's (macOS's
    /// `proc_pid_rusage`, as driver.cpp's `instructions`).
    #[cfg(target_os = "macos")]
    fn instructions() -> u64 {
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

    #[cfg(not(target_os = "macos"))]
    fn instructions() -> u64 {
        eprintln!("driver: check counts instructions with macOS's proc_pid_rusage, which this system lacks");
        std::process::exit(2);
    }

    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut std::ffi::c_void) -> i32;
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

    /// `check`'s second check, as driver.cpp's `small_op`.
    fn small_op(op: &str) -> bool {
        matches!(op, "update10th" | "select" | "swap" | "remove")
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
        let (mut i0, mut setup) = (0u64, 0u64);
        let mut model = Model::new();
        let failed = std::rc::Rc::new(std::cell::Cell::new(false));
        let failed_in = failed.clone();
        let hooks = Hooks {
            settled: Some(Box::new(move |s: &mut dyn Session| {
                if waiting {
                    let work = if check { instructions() - i0 } else { 0 };
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
                        println!("check {op} step {next} ({} row {}): {work} instructions", p.text, p.row);
                        if !timing && p.text == CREATE_1K {
                            setup = work;
                        }
                        let small = !timing || !small_op(&op) || work * 10 < setup;
                        if !small {
                            eprintln!("driver: check: {op} retired {work} instructions, not under a tenth of Create 1,000 rows' {setup}");
                        }
                        if !drawn_inside(s) || !small {
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
                    i0 = instructions();
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
