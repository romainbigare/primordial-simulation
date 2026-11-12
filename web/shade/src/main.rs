//! The page's shaders, written once in WGSL (`web/shaders/`), checked and translated for both of the map's back ends
//! (`web/src/gpu/`): WebGPU reads the WGSL as it is, WebGL2 reads GLSL ES 3.00 that naga (the shader translator of the
//! wgpu project) writes from it. A program is `web/shaders/<name>.wgsl` with the files of `web/shaders/lib/` its first
//! line names (`// uses: a.wgsl b.wgsl`) before it, and the entry points `vs` and `fs`. Every program starts with
//! `const LEAST_E` (draw.wgsl): the least metres a pixel takes in its draws, 0. A program with a line that names more
//! of them (`// least metres a pixel: 1 3`) is made once more for each, so the detail that cannot show at those sizes
//! of a pixel is compiled out of it (draw.wgsl `shows_at`); those differ from it in that constant alone, which the page
//! sets (`web/src/gpu.ts` `programs`, named `<name>:<metres>`).
//!
//! `ew-shade SHADERS_DIR OUT.json` writes, for each program: its WGSL; its GLSL vertex and fragment shaders; the least
//! metres a pixel takes for which it is made once more; and where
//! WebGL2 finds each of its bindings (group, binding): the name of each uniform block, and of each sampler with the
//! texture and the sampler it pairs (GLSL has no separate samplers).

use naga::back::glsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::{Handle, Module, ShaderStage};
use serde_json::{json, Map, Value};
use std::{env, fs, path::Path, process::ExitCode};

/// The line that names the files of `lib/` a program reads, before its own text.
const USES: &str = "// uses:";
/// The line that names the least metres a pixel takes for which a program is made once more.
const LEAST: &str = "// least metres a pixel:";

/// The program in `dir` named `name` for draws whose pixels take at least `least` metres: its own text with the files
/// of `lib/` it uses before it, after `LEAST_E`.
fn source(dir: &Path, name: &str, least: f32) -> Result<String, String> {
    let own = fs::read_to_string(dir.join(format!("{name}.wgsl"))).map_err(|e| format!("{name}: {e}"))?;
    let uses = own.lines().next().and_then(|l| l.strip_prefix(USES)).unwrap_or("");
    let mut text = format!("const LEAST_E: f32 = {least:?};\n");
    for lib in uses.split_whitespace() {
        text += &fs::read_to_string(dir.join("lib").join(lib)).map_err(|e| format!("{name}: {lib}: {e}"))?;
        text.push('\n');
    }
    Ok(text + &own)
}

/// The (group, binding) of a global, as JSON.
fn binding(module: &Module, h: Handle<naga::GlobalVariable>) -> Value {
    module.global_variables[h].binding.as_ref().map_or(Value::Null, |b| json!([b.group, b.binding]))
}

/// The GLSL ES 3.00 of entry point `entry` (stage `stage`) and where WebGL2 finds its bindings.
fn glsl_of(module: &Module, info: &naga::valid::ModuleInfo, stage: ShaderStage, entry: &str, binds: &mut Map<String, Value>) -> Result<String, String> {
    let options = glsl::Options {
        version: glsl::Version::Embedded { version: 300, is_webgl: true },
        writer_flags: glsl::WriterFlags::empty(),
        binding_map: Default::default(),
        zero_initialize_workgroup_memory: false,
    };
    let pipeline = glsl::PipelineOptions { shader_stage: stage, entry_point: entry.into(), multiview: None };
    let mut out = String::new();
    let mut writer = glsl::Writer::new(&mut out, module, info, &options, &pipeline, Default::default()).map_err(|e| format!("{entry}: {e}"))?;
    let reflection = writer.write().map_err(|e| format!("{entry}: {e}"))?;
    for (h, name) in &reflection.uniforms {
        binds.insert(name.clone(), json!({ "uniform": binding(module, *h) }));
    }
    for (name, m) in &reflection.texture_mapping {
        binds.insert(name.clone(), json!({ "texture": binding(module, m.texture), "sampler": m.sampler.map_or(Value::Null, |s| binding(module, s)) }));
    }
    Ok(out)
}

/// The least metres a pixel takes for which program `name` in `dir` is made once more (its line `LEAST`).
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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let [_, dir, out] = &args[..] else {
        eprintln!("usage: ew-shade SHADERS_DIR OUT.json");
        return ExitCode::FAILURE;
    };
    let written = programs(Path::new(dir)).and_then(|all| fs::write(out, Value::Object(all).to_string()).map_err(|e| format!("{out}: {e}")));
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shader of the page is valid WGSL and translates to GLSL ES 3.00 for both stages.
    #[test]
    fn every_program_translates() {
        let all = programs(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../shaders")).unwrap_or_else(|e| panic!("{e}"));
        assert!(all.contains_key("earth") && all.contains_key("cover"));
        // The live map is made once more for each size of a pixel its line `LEAST` names.
        assert!(all["land"]["least"].as_array().is_some_and(|l| l.contains(&json!("3.0"))));
        for (name, p) in &all {
            for stage in ["vertex", "fragment"] {
                assert!(p[stage].as_str().is_some_and(|g| g.starts_with("#version 300 es")), "{name} {stage}");
            }
        }
    }
}
