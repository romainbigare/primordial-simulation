import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a)
        s = s.replace(a, b)
    open(p, 'w').write(s)

edit('web/shade/src/main.rs', [
("""//! line names (`// uses: a.wgsl b.wgsl`) before it, and the entry points `vs` and `fs`.
//!""", """//! line names (`// uses: a.wgsl b.wgsl`) before it, and the entry points `vs` and `fs`. Every program starts with
//! `const LEAST_E` (draw.wgsl): the least metres a pixel takes in its draws, 0. A program whose second line names more
//! of them (`// least metres a pixel: 1 3`) is made once more for each, named `<name>:<metres>`, so the detail that
//! cannot show at those sizes of a pixel is compiled out of it (draw.wgsl `shows_at`).
//!"""),
("""/// The line that names the files of `lib/` a program reads, before its own text.
const USES: &str = "// uses:";
""", """/// The line that names the files of `lib/` a program reads, before its own text.
const USES: &str = "// uses:";
/// The line that names the least metres a pixel takes for which a program is made once more.
const LEAST: &str = "// least metres a pixel:";
"""),
("""/// The program in `dir` named `name`: its own text with the files of `lib/` it uses before it.
fn source(dir: &Path, name: &str) -> Result<String, String> {
    let own = fs::read_to_string(dir.join(format!("{name}.wgsl"))).map_err(|e| format!("{name}: {e}"))?;
    let uses = own.lines().next().and_then(|l| l.strip_prefix(USES)).unwrap_or("");
    let mut text = String::new();""", """/// The program in `dir` named `name` for draws whose pixels take at least `least` metres: its own text with the files
/// of `lib/` it uses before it, after `LEAST_E`.
fn source(dir: &Path, name: &str, least: f32) -> Result<String, String> {
    let own = fs::read_to_string(dir.join(format!("{name}.wgsl"))).map_err(|e| format!("{name}: {e}"))?;
    let uses = own.lines().next().and_then(|l| l.strip_prefix(USES)).unwrap_or("");
    let mut text = format!("const LEAST_E: f32 = {least:?};\\n");"""),
("""/// One program, checked and translated.
fn program(dir: &Path, name: &str) -> Result<Value, String> {
    let wgsl = source(dir, name)?;""", """/// The least metres a pixel takes for which program `name` in `dir` is made once more (its line `LEAST`), as written
/// and as a number.
fn leasts(dir: &Path, name: &str) -> Result<Vec<(String, f32)>, String> {
    let own = fs::read_to_string(dir.join(format!("{name}.wgsl"))).map_err(|e| format!("{name}: {e}"))?;
    let line = own.lines().find_map(|l| l.strip_prefix(LEAST)).unwrap_or("");
    line.split_whitespace().map(|m| m.parse::<f32>().map(|v| (m.to_string(), v)).map_err(|e| format!("{name}: {m}: {e}"))).collect()
}

/// One program for pixels of at least `least` metres, checked and translated.
fn program(dir: &Path, name: &str, least: f32) -> Result<Value, String> {
    let wgsl = source(dir, name, least)?;"""),
("""    names.sort();
    names.into_iter().map(|name| Ok((name.clone(), program(dir, &name)?))).collect()""",
"""    names.sort();
    let mut all = Map::new();
    for name in names {
        all.insert(name.clone(), program(dir, &name, 0.0)?);
        for (m, least) in leasts(dir, &name)? {
            all.insert(format!("{name}:{m}"), program(dir, &name, least)?);
        }
    }
    Ok(all)"""),
])
