//! `ew-analyze TAPE_DIR [--out cards.json]`: read a run's folder and write its fact cards (docs/analyze.md).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dir, out) = match args.as_slice() {
        [dir] => (dir, None),
        [dir, flag, out] if flag == "--out" => (dir, Some(out)),
        _ => {
            eprintln!("usage: ew-analyze TAPE_DIR [--out cards.json]");
            std::process::exit(2);
        }
    };
    let report = analyze::analyze(dir.as_ref()).unwrap_or_else(|e| {
        eprintln!("ew-analyze: {e}");
        std::process::exit(1);
    });
    let text = serde_json::to_string_pretty(&report).expect("cards are JSON") + "\n";
    match out {
        Some(path) => std::fs::write(path, text).unwrap_or_else(|e| panic!("write {path}: {e}")),
        None => print!("{text}"),
    }
    eprintln!("{} cards", report["cards"].as_array().map_or(0, Vec::len));
}
