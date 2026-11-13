"""PROBE (never committed): the radiation budget of a world, summed over the steps, with the clear sky's outgoing heat
light and the window. s3/rad_toggle5.py on|off (S3b: clear and cloudy skies)"""
import sys, shutil, os
root = '/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c/'
S = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/'
on = sys.argv[1] == 'on'
DEF = """/// PROBE: [sun at the top, reflected by clouds, absorbed at the surface (with the cover), surface heat light to space,
/// surface heat light up, heat light down at the surface, the air's heat light to space, area x time, surface reflected,
/// absorbed in the air, reflected by the air, to space, clear: surface to space, window to space, air to space, the
/// clouds' share of the sky x area x time, that x the cloud's depth]
pub static PROBE_R: std::sync::Mutex<[f64; 20]> = std::sync::Mutex::new([0.0; 20]);
/// PROBE: per air node its clear column and its clear window band alone.
pub static PROBE_CLEAR: std::sync::Mutex<Vec<[Heat; 2]>> = std::sync::Mutex::new(Vec::new());
"""
ANCH = "/// What each air node's air does to heat radiation (`Skies`): its gases that absorb it, each with the height its\n"
L_END = """            Skies::of(sky(0.0), (share > 0.0).then(|| (share, sky(radiation::drops_heat(depth)))))
        })
        .collect();
"""
L_ADD = """    *PROBE_CLEAR.lock().unwrap() = (0..na).map(|j| { // PROBE
        let gases: Vec<(usize, f64, f64)> = s.state.gas.iter().enumerate().filter(|e| chem.absorbs_heat(*e.1)).map(|(gi, &k)| (k, w.store.amounts(Class::Air)[k * na + j] / air.area()[j], s.hk[gi * na + j].max(COLUMN_MIN))).collect(); // PROBE
        let big = node_scale_height(s, j, g); // PROBE
        let b = band_depths(chem, &gases, (s.ta[j], big)); // PROBE
        let col = (s.gam[j], (top - s.za[j]).max(COLUMN_MIN)); // PROBE
        [Heat::of(b, (0.0, big), col, w.cfg.line_spread), Heat::of([b[0], (0.0, big), (0.0, big)], (0.0, big), col, w.cfg.line_spread)] // PROBE
    }).collect(); // PROBE
"""
C1 = "    let per_area = if cover > 0.0 { cover / a } else { 1.0 };\n"
C1A = """    { let ts = p[TO_SPACE * (2 + act.len())]; let refl = (1.0 - cover) * alb + cover * (1.0 - g.absorb); let sh = cx.light[j].shares(refl); let clr = cx.light[j].probe_clear(); let up = if rad.f < 1.0 { ts / (1.0 - rad.f) } else { 0.0 }; let cl = super::PROBE_CLEAR.lock().unwrap()[j]; let mut r = super::PROBE_R.lock().unwrap(); r[0] += g.sun * adt; r[1] += g.sun * adt * (sh[2] - clr.shares(refl)[2]); r[2] += adt * sw + kept * adt; r[3] += ts; r[4] += up; r[5] += rad.down * adt; r[7] += adt; r[8] += adt * sun - adt * sw - kept * adt; r[9] += adt * top * sh[1]; r[10] += adt * top * clr.shares(0.0)[2]; r[11] += adt * top * sh[2]; r[12] += up * (1.0 - cl[0].absorbs(&w.chem, g.tg)); r[13] += up * (w.chem.heat_shares(g.tg)[0] - cl[1].absorbs(&w.chem, g.tg)); r[17] += adt * top * clr.shares(refl)[1]; } // PROBE
"""
D1 = "        out.space[j] += sky;\n"
D1A = """        { let c = super::PROBE_CLEAR.lock().unwrap()[j][0].glow(&w.chem, ts[j], UP)[0] * air.area()[j] * dd; let mut r = super::PROBE_R.lock().unwrap(); r[6] += sky; r[14] += c; } // PROBE
"""
B1 = "    let ema = year_weight(w, dt);\n"
B1A = "    { let mut r = super::PROBE_R.lock().unwrap(); for j in 0..na { let [b, tau] = s.state.cloud[j]; let a = w.mesh.level(AIR).area()[j] * dt; r[15] += b * a; r[16] += b * tau * a; } } // PROBE\n"
R1 = "        Light { clear, cloud }\n    }\n"
R1A = "    pub(crate) fn probe_clear(&self) -> Column { self.clear } // PROBE\n"
EXP_A = "pub use air::{band_depths, column, declination, insolation, on_slope, scale_height, sun_by_latitude, Heat, Planet, DOWN, UP};\n"
EXP_B = "pub use air::PROBE_R; // PROBE\n"


def edit(path, a, b):
    p = root + path
    s = open(p).read()
    assert s.count(a) == 1, (path, a[:60])
    open(p, 'w').write(s.replace(a, b))


if on:
    edit('core/src/flow/air/mod.rs', ANCH, DEF + ANCH)
    edit('core/src/flow/air/mod.rs', L_END, L_END + L_ADD)
    edit('core/src/flow/air/cells.rs', C1, C1A + C1)
    edit('core/src/flow/air/daily.rs', D1, D1 + D1A)
    edit('core/src/flow/mod.rs', EXP_A, EXP_A + EXP_B)
    edit('core/src/flow/air/book.rs', B1, B1A + B1)
    edit('core/src/flow/air/radiation.rs', R1, R1 + R1A)
    os.makedirs(root + 'cli/examples', exist_ok=True)
    shutil.copy(S + 's3/radprobe4.rs', root + 'cli/examples/radprobe.rs')
else:
    edit('core/src/flow/air/mod.rs', DEF, '')
    edit('core/src/flow/air/mod.rs', L_ADD, '')
    edit('core/src/flow/air/cells.rs', C1A, '')
    edit('core/src/flow/air/daily.rs', D1A, '')
    edit('core/src/flow/mod.rs', EXP_B, '')
    edit('core/src/flow/air/book.rs', B1A, '')
    edit('core/src/flow/air/radiation.rs', R1A, '')
    if os.path.exists(root + 'cli/examples/radprobe.rs'):
        os.remove(root + 'cli/examples/radprobe.rs')
        if not os.listdir(root + 'cli/examples'):
            os.rmdir(root + 'cli/examples')
