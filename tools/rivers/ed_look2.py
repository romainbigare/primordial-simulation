p = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv/look/src/main.rs'
t = open(p).read()
a = """            if std::env::var("SHADE").is_ok() {"""
b = """            if let Ok(row) = std::env::var("ROW") {
                let r: Vec<usize> = row.split(',').map(|v| v.parse().unwrap()).collect();
                if j == r[0] && i >= r[1] && i < r[2] { println!("{i}: h {:.3} sd {:.2} calm {:.2}", at(0, 0, 0, 0) + at(0, 0, 0, 1), at(0, 0, 0, 3), at(0, 0, 1, 3)); }
            }
            if std::env::var("SHADE").is_ok() {"""
assert t.count(a) == 1
open(p, 'w').write(t.replace(a, b))
