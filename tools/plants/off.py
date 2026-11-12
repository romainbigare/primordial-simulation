import re, sys
W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
p = W + 'web/shaders/lib/cover_field.wgsl'
s = open(p).read()
m = re.search(r'fn cover_field\([^)]*\) -> (\w+) \{\n', s)
ret = 'CoverAt(vec4f(0.0), 0.0)' if m.group(1) == 'CoverAt' else 'vec4f(0.0)'
s = s[:m.end()] + '  if (true) { return ' + ret + '; }\n' + s[m.end():]
open(p, 'w').write(s)
p = W + 'web/src/textures.ts'
s = open(p).read()
a = '  draw(gpu: Gpu, p: Pass, m: Marks): void {\n'
assert a in s
s = s.replace(a, a + '    if (m) return;\n')
open(p, 'w').write(s)
print('off', m.group(1))
