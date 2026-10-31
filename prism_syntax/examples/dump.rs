//! `cargo run -p prism_syntax --example dump -- [--tokens|--pretty|--fragment] FILE`:
//! prints a file's tree (the S-expression dump), its tokens in the layout corpus's
//! notation, or its pretty-printed form, then its diagnostics.

use prism_syntax::{corpus, diag::render_all, lex, parse_fragment, parse_module, pretty, sexp};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let file = args.iter().find(|a| !a.starts_with("--")).expect("a file");
    let src = std::fs::read_to_string(file).expect("readable file");
    if args.iter().any(|a| a == "--tokens") {
        let l = lex(&src);
        print!("{}", corpus::render(&l.tokens));
        println!("{}", render_all(&src, &l.diagnostics));
        return;
    }
    let parsed = if args.iter().any(|a| a == "--fragment") { parse_fragment(&src) } else { parse_module(&src) };
    if args.iter().any(|a| a == "--pretty") {
        print!("{}", pretty::module(&parsed.tree));
    } else {
        print!("{}", sexp::module(&parsed.tree));
    }
    println!("{}", render_all(&src, &parsed.diagnostics));
}
