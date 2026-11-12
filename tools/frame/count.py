# Instrument a copy of web/shaders to count work per pixel: every texture read and every random word (lowbias, pcg3d).
# The program named (land or cover) writes its counts as its colour; the others write black (cover: nothing).
# R = reads % 256, G = words % 256, B = reads / 256 + 16 * (words / 256).
# python3 count.py SHADERS_DIR land|cover
import os, re, sys
d, which = sys.argv[1], sys.argv[2]

def wrap_calls(s, name):
    out, i = [], 0
    while True:
        j = s.find(name + '(', i)
        if j < 0:
            out.append(s[i:]); break
        out.append(s[i:j])
        k, depth = j + len(name), 0
        while True:
            ch = s[k]
            if ch == '(':
                depth += 1
            elif ch == ')':
                depth -= 1
                if depth == 0:
                    break
            k += 1
        out.append('wk_t(' + s[j:k + 1] + ')')
        i = k + 1
    return ''.join(out)

for root, _, files in os.walk(d):
    for f in files:
        if not f.endswith('.wgsl'):
            continue
        p = os.path.join(root, f)
        s = open(p).read()
        for n in ('textureLoad', 'textureSampleLevel'):
            s = wrap_calls(s, n)
        if f == 'draw.wgsl' and 'lib' in root:
            s += '\nvar<private> wk: vec2f;\nfn wk_t(v: vec4f) -> vec4f { wk.x += 1.0; return v; }\n'
            s += 'fn wk_out() -> vec4f { let r = floor(wk.x + 0.5); let w = floor(wk.y + 0.5);\n'
            s += '  return vec4f((r % 256.0) / 255.0, (w % 256.0) / 255.0, (floor(r / 256.0) + 16.0 * floor(w / 256.0)) / 255.0, 1.0); }\n'
            s = s.replace('fn lowbias(c: vec2i, k: i32) -> u32 {\n', 'fn lowbias(c: vec2i, k: i32) -> u32 {\n  wk.y += 1.0;\n')
            assert 'wk.y += 1.0' in s
        if f == 'cover.wgsl':
            s = s.replace('fn pcg3d(w: vec3u) -> vec3u {\n', 'fn pcg3d(w: vec3u) -> vec3u {\n  wk.y += 1.0;\n')
            assert 'wk.y += 1.0' in s
        if f == 'backdrop.wgsl':
            s = re.sub(r'return (vec4f\([^;]*\));\n\}\s*$', 'return vec4f(0.0, 0.0, 0.0, 1.0);\n}\n', s)
        if f in ('land.wgsl', 'cover.wgsl') and 'lib' not in root:
            me = f.startswith(which)
            # The fragment stage's returns.
            m = s.index('@fragment fn fs')
            body = s[m:]
            if me:
                body = re.sub(r'return [^;]*;', 'return wk_out();', body)
            else:
                body = re.sub(r'return [^;]*;', 'return vec4f(0.0, 0.0, 0.0, %s);' % ('1.0' if f == 'land.wgsl' else '0.0'), body)
            s = s[:m] + body
        open(p, 'w').write(s)
