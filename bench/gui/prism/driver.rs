// The Prism table's driver for the Rust target (bench/gui/README.md), driver.cpp in Rust:
// bench.prism compiled to Rust runs under the Rust canvas World with `Hooks`, and each
// step of an operation's script is a press at a button's text, found in the scene the
// World last read. A timed step's clock starts when the press is queued and stops at the
// next `Hooks::settled`: every tick it caused has run and every window has presented.
//
//   driver OP RUNS WARMUP     prints `OP RUN MS` per timed run, then `peak_rss_bytes N`
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

    /// What the screen must show after an operation's timed step: the rows (one "x"
    /// each), and a text that must start or end with the marker, if any.
    fn expected(op: &str) -> (usize, &'static str) {
        match op {
            "update10th" => (1000, " !!!"),
            "select" => (1000, "\u{25b6} "),
            "remove" => (999, ""),
            "create10k" => (10000, ""),
            "append1k" => (2000, ""),
            "clear" => (0, ""),
            _ => (1000, ""),
        }
    }

    /// Whether the screen shows what the operation asks for; says what it saw when not.
    fn shows(s: &dyn Session, op: &str) -> bool {
        let (want, marker) = expected(op);
        let mut all = Vec::new();
        for scene in s.shared().scenes.values() {
            texts(&scene.draw, &mut all);
        }
        let rows = all.iter().filter(|(_, _, t)| t == "x").count();
        let marked = marker.is_empty() || all.iter().any(|(_, _, t)| t.starts_with(marker) || t.ends_with(marker));
        if rows == want && marked {
            return true;
        }
        eprintln!("driver: the screen shows {rows} rows (wanted {want}){}", if marked { "" } else { ", and no marked label" });
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
        let (op, runs, warmup) = match args.as_slice() {
            [op, r, w] => (op.clone(), r.parse::<usize>().unwrap_or(0), w.parse::<usize>().ok()),
            _ => (String::new(), 0, None),
        };
        let (Some(one), Some(warmup)) = (script(&op), warmup) else {
            eprintln!("usage: driver OP RUNS WARMUP (OP: create1k replace1k update10th select swap remove create10k append1k clear)");
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
        let failed = std::rc::Rc::new(std::cell::Cell::new(false));
        let failed_in = failed.clone();
        let hooks = Hooks {
            settled: Some(Box::new(move |s: &mut dyn Session| {
                if waiting && timing {
                    run += 1;
                    println!("{op} {run} {:.3}", t0.elapsed().as_secs_f64() * 1e3);
                    if !shows(s, &op) {
                        failed_in.set(true);
                        s.exit(1);
                        return;
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
