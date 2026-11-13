p = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv/look/src/main.rs'
t = open(p).read()
a = """            if std::env::var("CALM").is_ok() {"""
b = """            if std::env::var("SHADE").is_ok() {
                // The ground's light alone: its slope east and south.
                let hh = |dx: i64, dy: i64| at(dx, dy, 0, 0) + at(dx, dy, 0, 1);
                let s = (0.5 + 1.5 * ((hh(1, 0) - hh(0, 0)) + (hh(0, 1) - hh(0, 0))) / texel).clamp(0.0, 1.0);
                c = [255.0 * s, 255.0 * s, 255.0 * s];
            } else if std::env::var("CALM").is_ok() {"""
assert t.count(a) == 1
open(p, 'w').write(t.replace(a, b))
