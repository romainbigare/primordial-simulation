"""PROBE (never committed): per cell, the surface's budget summed over a month (sunlight absorbed, heat light up, back
light all-sky and clear-sky, the cloud's share, contact heat, latent heat, the surface's light to space, the sunlight
its air absorbs), and per air node its light to space. polar_toggle.py on|off tip|main"""
import sys, os, shutil
root = '/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c/'
S = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/final/tools/'
on, tree = sys.argv[1] == 'on', sys.argv[2]

DEF = """/// PROBE (never committed)
pub static PROBE_CELL: std::sync::Mutex<Vec<[f64; 11]>> = std::sync::Mutex::new(Vec::new());
/// PROBE (never committed)
pub static PROBE_SPACE: std::sync::Mutex<Vec<f64>> = std::sync::Mutex::new(Vec::new());
/// PROBE (never committed)
pub fn probe_add(i: usize, v: [f64; 10], adt: f64) { let mut m = PROBE_CELL.lock().unwrap(); if m.len() <= i { m.resize(i + 1, [0.0; 11]); } for k in 0..10 { m[i][k] += v[k] * adt; } m[i][10] += adt; }
/// PROBE (never committed)
pub fn probe_space(j: usize, x: f64) { let mut m = PROBE_SPACE.lock().unwrap(); if m.len() <= j { m.resize(j + 1, 0.0); } m[j] += x; }
/// PROBE (never committed)
pub static PROBE_NODE: std::sync::Mutex<Vec<[f64; 9]>> = std::sync::Mutex::new(Vec::new());
/// PROBE (never committed)
pub fn probe_node(j: usize, v: [f64; 8], wt: f64) { let mut m = PROBE_NODE.lock().unwrap(); if m.len() <= j { m.resize(j + 1, [0.0; 9]); } for k in 0..8 { m[j][k] += v[k] * wt; } m[j][8] += wt; }
"""
FLOW_ANCH = "pub fn state(w: &mut World, full: bool) {"
CELL_ANCH = "    let per_area = if cover > 0.0 { cover / a } else { 1.0 };\n"
if tree == 'tip':
    CELL_ADD = """    { let bal = solve(sw); let t3 = 5.670374419e-8 * g.tg.powi(3); let up = t3 * (4.0 * bal.ts - 3.0 * g.tg); let la = gas.iter().zip(ev.iter()).map(|(q, e)| q.lat(e[0])).sum::<f64>() / adt; let (cd, sh) = s.heat[j].probe_clear(&w.chem, tl); crate::flow::probe_add(g.cell, [sw, up, rad.down, cd, sh, w.cfg.sensible * (bal.ts - tl), la, bal.to_space / adt, top * in_air, bal.ts], adt); } // PROBE
"""
else:
    CELL_ADD = """    { let swv = sw(alb); let bal = solve(swv); let t3 = 5.670374419e-8 * g.tg.powi(3); let up = t3 * (4.0 * bal.ts - 3.0 * g.tg); let la = gas.iter().zip(ev.iter()).map(|(q, e)| q.lat(e[0])).sum::<f64>() / adt; crate::flow::probe_add(g.cell, [swv, up, rad.down, f64::NAN, s.state.cloud[j][0], w.cfg.sensible * (bal.ts - tl), la, bal.to_space / adt, 0.0, bal.ts], adt); } // PROBE
"""
D1 = "        out.space[j] += sky;\n"
D1A = "        crate::flow::probe_space(j, sky); // PROBE\n"
N_ANCH = "        *c += act.iter().enumerate().filter(|p| p.1.cond).map(of).fold(0.0, f64::max) * share;\n"
N_ADD = """        if let Some(ac) = act.iter().find(|a| a.cond) { let full = saturating(w, s, ac, j, ts[j]); crate::flow::probe_node(j, [mean[ac.gi * na + j] / full.max(1e-300), ts[j], s.hk[ac.gi * na + j], s.gam[j], s.za[j], s.zair[j], s.state.cloud[j][1], act.iter().enumerate().filter(|p| p.1.cond).map(of).fold(0.0, f64::max)], share); } // PROBE
"""
R_ANCH = "    /// `Heat::absorbs` of the whole sky.\n"
R_ADD = """    /// PROBE (never committed)
    pub(crate) fn probe_clear(&self, chem: &Chemistry, tl: f64) -> (f64, f64) { (self.clear.glow(chem, tl, DOWN)[0], self.cloud.as_ref().map_or(0.0, |c| c.0)) }
"""

def edit(path, a, b):
    p = root + path
    s = open(p).read()
    assert s.count(a) == 1, (path, a[:60], s.count(a))
    open(p, 'w').write(s.replace(a, b))

if on:
    edit('core/src/flow/mod.rs', FLOW_ANCH, DEF + FLOW_ANCH)
    edit('core/src/flow/air/cells.rs', CELL_ANCH, CELL_ADD + CELL_ANCH)
    edit('core/src/flow/air/daily.rs', D1, D1 + D1A)
    if tree == 'tip':
        edit('core/src/flow/air/radiation.rs', R_ANCH, R_ADD + R_ANCH)
        edit('core/src/flow/air/daily.rs', N_ANCH, N_ANCH + N_ADD)
    os.makedirs(root + 'cli/examples', exist_ok=True)
    shutil.copy(S + 'globeprobe.rs', root + 'cli/examples/globeprobe.rs')
else:
    edit('core/src/flow/mod.rs', DEF, '')
    edit('core/src/flow/air/cells.rs', CELL_ADD, '')
    edit('core/src/flow/air/daily.rs', D1A, '')
    if tree == 'tip':
        edit('core/src/flow/air/radiation.rs', R_ADD, '')
        edit('core/src/flow/air/daily.rs', N_ADD, '')
    shutil.rmtree(root + 'cli/examples', ignore_errors=True)
