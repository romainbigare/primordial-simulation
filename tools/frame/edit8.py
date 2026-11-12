import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a)
        s = s.replace(a, b)
    open(p, 'w').write(s)
p = 'web/shade/src/main.rs'
s = open(p).read()
i = s.index('/// The least metres a pixel takes for which program `name` in `dir` is made once more')
j = s.index('fn main() -> ExitCode {')
s = s[:i] + '''/// The least metres a pixel takes for which program `name` in `dir` is made once more (its line `LEAST`).
fn leasts(dir: &Path, name: &str) -> Result<Vec<f32>, String> {
    let own = fs::read_to_string(dir.join(format!("{name}.wgsl"))).map_err(|e| format!("{name}: {e}"))?;
    let line = own.lines().find_map(|l| l.strip_prefix(LEAST)).unwrap_or("");
    line.split_whitespace().map(|m| m.parse::<f32>().map_err(|e| format!("{name}: {m}: {e}"))).collect()
}

/// Text of a program made for no least size of a pixel, made for `m` metres: its declaration of `LEAST_E` (WGSL
/// `LEAST_E: f32 = 0.0;`, GLSL `LEAST_E = 0.0;`) given that value, as the page does (web/src/gpu.ts `programs`).
fn made_for(text: &str, m: &str) -> String {
    text.replacen("LEAST_E: f32 = 0.0;", &format!("LEAST_E: f32 = {m};"), 1).replacen("LEAST_E = 0.0;", &format!("LEAST_E = {m};"), 1)
}

/// Program `name` for pixels of at least `least` metres, checked and translated.
fn translated(dir: &Path, name: &str, least: f32) -> Result<Value, String> {
    let wgsl = source(dir, name, least)?;
    let module = naga::front::wgsl::parse_str(&wgsl).map_err(|e| e.emit_to_string_with_path(&wgsl, &format!("{name}.wgsl")))?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .map_err(|e| e.emit_to_string_with_path(&wgsl, &format!("{name}.wgsl")))?;
    let mut binds = Map::new();
    let vertex = glsl_of(&module, &info, ShaderStage::Vertex, "vs", &mut binds)?;
    let fragment = glsl_of(&module, &info, ShaderStage::Fragment, "fs", &mut binds)?;
    Ok(json!({ "wgsl": wgsl, "vertex": vertex, "fragment": fragment, "binds": binds }))
}

/// One program, checked and translated, with the least metres a pixel takes for which it is made once more (`least`,
/// as the declaration of `LEAST_E` writes them): each of those is checked and translated too, and must differ from it
/// in that declaration alone, so the page makes it from this one (`made_for`) and holds one text of each program.
fn program(dir: &Path, name: &str) -> Result<Value, String> {
    let mut base = translated(dir, name, 0.0)?;
    let mut least = Vec::new();
    for m in leasts(dir, name)? {
        let made = translated(dir, name, m)?;
        let written = format!("{m:?}");
        for part in ["wgsl", "vertex", "fragment"] {
            if made[part].as_str() != Some(made_for(base[part].as_str().unwrap_or(""), &written).as_str()) {
                return Err(format!("{name}: its {part} made for {written} m a pixel differs in more than LEAST_E"));
            }
        }
        least.push(written);
    }
    base["least"] = json!(least);
    Ok(base)
}

/// Every program in `dir`, by name.
fn programs(dir: &Path) -> Result<Map<String, Value>, String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".wgsl").map(String::from))
        .collect();
    names.sort();
    names.into_iter().map(|name| Ok((name.clone(), program(dir, &name)?))).collect()
}

''' + s[j:]
s = s.replace('''//! of them (`// least metres a pixel: 1 3`) is made once more for each, named `<name>:<metres>`, so the detail that
//! cannot show at those sizes of a pixel is compiled out of it (draw.wgsl `shows_at`).''', '''//! of them (`// least metres a pixel: 1 3`) is made once more for each, so the detail that cannot show at those sizes
//! of a pixel is compiled out of it (draw.wgsl `shows_at`); those differ from it in that constant alone, which the page
//! sets (`web/src/gpu.ts` `programs`, named `<name>:<metres>`).''')
s = s.replace('''        // The live map is made once more for each size of a pixel its line `LEAST` names.
        assert!(all["land:3"]["wgsl"].as_str().is_some_and(|w| w.starts_with("const LEAST_E: f32 = 3.0;")));''',
'''        // The live map is made once more for each size of a pixel its line `LEAST` names.
        assert!(all["land"]["least"].as_array().is_some_and(|l| l.contains(&json!("3.0"))));''')
s = s.replace('''/// The line that names the least metres a pixel takes for which a program is made once more.''', '''/// The line that names the least metres a pixel takes for which a program is made once more.''')
s = s.replace('''//! `ew-shade SHADERS_DIR OUT.json` writes, for each program: its WGSL; its GLSL vertex and fragment shaders; and where''',
'''//! `ew-shade SHADERS_DIR OUT.json` writes, for each program: its WGSL; its GLSL vertex and fragment shaders; the least
//! metres a pixel takes for which it is made once more; and where''')
open(p, 'w').write(s)

edit('web/src/gpu.ts', [
("""/** A program as web/shade writes it: its WGSL, its GLSL stages and the GLSL names of its bindings. */
export interface Program { wgsl: string; vertex: string; fragment: string; binds: Record<string, Bind> }
declare const SHADERS: Record<string, Program>;
/** The page's programs (web/shaders/<name>.wgsl). */
export const programs = (): Record<string, Program> => SHADERS;""",
"""/** A program as web/shade writes it: its WGSL, its GLSL stages, the GLSL names of its bindings, and the least metres a
 * pixel takes for which it is made once more (as its declaration of `LEAST_E` writes them). */
export interface Program { wgsl: string; vertex: string; fragment: string; binds: Record<string, Bind>; least: string[] }
declare const SHADERS: Record<string, Program>;
/** The declaration of `LEAST_E` in a program made for no least size of a pixel (WGSL and GLSL; web/shaders/lib/draw.wgsl),
 * and its value. */
const LEAST_DECLARED = /(LEAST_E(?:: f32)? = )0\\.0;/;
let all: Record<string, Program> | null = null;
/** The page's programs (web/shaders/<name>.wgsl), and each one made for draws whose pixels take at least `m` metres,
 * named `<name>:<m>`: the same text with `LEAST_E` set to `m` (web/shade checks that this is what it makes). */
export function programs(): Record<string, Program> {
  if (all) return all;
  const out: Record<string, Program> = { ...SHADERS };
  for (const [name, p] of Object.entries(SHADERS)) {
    for (const m of p.least) {
      const set = (text: string) => text.replace(LEAST_DECLARED, `$1${m};`);
      out[`${name}:${m}`] = { ...p, wgsl: set(p.wgsl), vertex: set(p.vertex), fragment: set(p.fragment), least: [] };
    }
  }
  return (all = out);
}"""),
("""  for (const key of Object.keys(SHADERS)) {""", """  for (const key of Object.keys(programs())) {"""),
])
