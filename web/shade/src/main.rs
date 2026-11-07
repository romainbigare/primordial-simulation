//! The page's shaders, written once in WGSL (`web/shaders/`), checked and translated for both of the map's back ends
//! (`web/src/gpu/`): WebGPU reads the WGSL as it is, WebGL2 reads GLSL ES 3.00 that naga (the shader translator of the
//! wgpu project) writes from it. A program is `web/shaders/<name>.wgsl` with the files of `web/shaders/lib/` its first
//! line names (`// uses: a.wgsl b.wgsl`) before it, and the entry points `vs` and `fs`.
//!
//! `ew-shade SHADERS_DIR OUT.json` writes, for each program: its WGSL; its GLSL vertex and fragment shaders; and where
//! WebGL2 finds each of its bindings (group, binding): the name of each uniform block, and of each sampler with the
//! texture and the sampler it pairs (GLSL has no separate samplers).

use naga::back::glsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::{Handle, Module, ShaderStage};
use serde_json::{json, Map, Value};
use std::{env, fs, path::Path, process::ExitCode};

/// The line that names the files of `lib/` a program reads, before its own text.
const USES: &str = "// uses:";

/// The program in `dir` named `name`: its own text with the files of `lib/` it uses before it.
fn source(dir: &Path, name: &str) -> Result<String, String> {
    let own = fs::read_to_string(dir.join(format!("{name}.wgsl"))).map_err(|e| format!("{name}: {e}"))?;
    let uses = own.lines().next().and_then(|l| l.strip_prefix(USES)).unwrap_or("");
    let mut text = String::new();
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

/// One program, checked and translated.
fn program(dir: &Path, name: &str) -> Result<Value, String> {
    let wgsl = source(dir, name)?;
    let module = naga::front::wgsl::parse_str(&wgsl).map_err(|e| e.emit_to_string_with_path(&wgsl, &format!("{name}.wgsl")))?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .map_err(|e| e.emit_to_string_with_path(&wgsl, &format!("{name}.wgsl")))?;
    let mut binds = Map::new();
    let vertex = glsl_of(&module, &info, ShaderStage::Vertex, "vs", &mut binds)?;
    let fragment = glsl_of(&module, &info, ShaderStage::Fragment, "fs", &mut binds)?;
    Ok(json!({ "wgsl": wgsl, "vertex": vertex, "fragment": fragment, "binds": binds }))
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
        for (name, p) in &all {
            for stage in ["vertex", "fragment"] {
                assert!(p[stage].as_str().is_some_and(|g| g.starts_with("#version 300 es")), "{name} {stage}");
            }
        }
    }
}
