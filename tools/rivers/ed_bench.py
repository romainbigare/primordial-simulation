p = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv/bench.js'
t = open(p).read()
a = "const cpuOf = pid => {"
b = "// The CPU time of the whole tree of processes under this one (the browser and all its children), ms.\nconst treeCpu = () => { const all = [String(process.pid)]; for (let i = 0; i < all.length; i++) all.push(...kids(all[i])); return all.slice(1).reduce((s, d) => { try { return s + cpuOf(d); } catch { return s; } }, 0); };\nconst cpuOf = pid => {"
assert t.count(a) == 1
t = t.replace(a, b)
for x, y in [("    const gpuPid = gpuProcess(process.pid);\n", ""), ("const c0 = cpuOf(gpuPid);", "const c0 = treeCpu();"), ("t.cpu = cpuOf(gpuPid) - c0;", "t.cpu = treeCpu() - c0;")]:
    assert t.count(x) == 1, x
    t = t.replace(x, y)
open(p, 'w').write(t)
