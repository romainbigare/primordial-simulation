"""PROBE (never committed): the radiation budget of a world, summed over the steps. rad_toggle.py on|off"""
import sys, shutil, os
root = '/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c/'
S = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/'
on = sys.argv[1] == 'on'
DEF = """/// PROBE: [sun at the top, reflected by clouds, absorbed at the surface (with the cover), surface heat light to space,
/// surface heat light up, heat light down at the surface, the air's heat light to space, area x time, surface reflected]
pub static PROBE_R: std::sync::Mutex<[f64; 12]> = std::sync::Mutex::new([0.0; 12]);
"""
ANCH = "/// The layers of each air node's column that send its heat radiation"
C1 = "    let per_area = if cover > 0.0 { cover / a } else { 1.0 };\n"
C1A = """    { let ts = p[TO_SPACE * (2 + act.len())]; let sh = shares(alb); let mut r = super::PROBE_R.lock().unwrap(); r[0] += g.sun * adt; r[1] += g.sun * s.state.cloud[j][0] * adt; r[2] += adt * sw(alb) + kept(alb) * adt; r[3] += ts; r[4] += if f < 1.0 { ts / (1.0 - f) } else { 0.0 }; r[5] += rad.down * adt; r[7] += adt; r[8] += adt * sun(alb) - adt * sw(alb) - kept(alb) * adt; r[9] += adt * top * sh[1]; r[10] += adt * top * cx.light[j].shares(0.0)[2]; r[11] += adt * top * sh[2]; } // PROBE
"""
D1 = "        out.space[j] += sky;\n"
D1A = "        super::PROBE_R.lock().unwrap()[6] += sky; // PROBE\n"
EXP_A = "pub use air::{column, declination, glow, insolation, on_slope, scale_height, sun_by_latitude, Planet};\n"
EXP_B = "pub use air::PROBE_R; // PROBE\n"

def edit(path, f):
    p = root + path
    s = open(p).read()
    t = f(s)
    open(p, 'w').write(t)

if on:
    edit('core/src/flow/air/mod.rs', lambda s: s.replace(ANCH, DEF + ANCH, 1))
    edit('core/src/flow/air/cells.rs', lambda s: s.replace(C1, C1A + C1, 1))
    edit('core/src/flow/air/daily.rs', lambda s: s.replace(D1, D1 + D1A, 1))
    edit('core/src/flow/mod.rs', lambda s: s.replace(EXP_A, EXP_A + EXP_B, 1))
    os.makedirs(root + 'cli/examples', exist_ok=True)
    shutil.copy(S + 'radprobe.rs', root + 'cli/examples/radprobe.rs')
else:
    edit('core/src/flow/air/mod.rs', lambda s: s.replace(DEF, ''))
    edit('core/src/flow/air/cells.rs', lambda s: s.replace(C1A, ''))
    edit('core/src/flow/air/daily.rs', lambda s: s.replace(D1A, ''))
    edit('core/src/flow/mod.rs', lambda s: s.replace(EXP_B, ''))
    if os.path.exists(root + 'cli/examples/radprobe.rs'):
        os.remove(root + 'cli/examples/radprobe.rs')
        if not os.listdir(root + 'cli/examples'):
            os.rmdir(root + 'cli/examples')
