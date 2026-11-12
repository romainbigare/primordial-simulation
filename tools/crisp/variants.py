# Copies of the measuring page (perf-now) with one part of the drawing switched off each, by text in its shaders.
import os, shutil
base = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/crisp/'
cuts = {
    'norelief': ('shows_at(RELIEF_LONGEST_M, e) <= 0.0', 'true'),
    'nobars': ('noise_turned(at / (2.0 * WANDER * banks), 63)', '0.0'),
    'nopool': ('spots.pool > 0.0 && spots.pool < 1.0', 'false'),
    'nopatch': ('mean * (1.0 - PATCHY) >= max(HIDES_BED_M, SMOOTH_LOOSE_M)', 'true'),
}
src = open(base + 'perf-now/index.html').read()
for name, (a, b) in cuts.items():
    assert src.count(a) >= 1, name
    d = base + 'perf-' + name
    os.makedirs(d, exist_ok=True)
    shutil.copy(base + 'perf-now/ew.wasm', d + '/ew.wasm')
    open(d + '/index.html', 'w').write(src.replace(a, b))
print('ok')
