# Copy the worktree's changed files to a folder, or back: python3 keep.py save|load DIR
import os, shutil, sys
FILES = """tape/src/ground/bends.rs tape/src/ground/mod.rs tape/src/ground/rock.rs web/shaders/land.wgsl web/shaders/lib/colour.wgsl
web/shaders/lib/crag.wgsl web/shaders/lib/earth.wgsl web/shaders/lib/land.wgsl web/shaders/lib/river.wgsl web/shaders/lib/rock.wgsl
web/shaders/lib/stone.wgsl web/shaders/lib/flow.wgsl web/src/land.ts web/src/map.ts docs/numbers.md docs/tape.md README.md""".split()
mode, d = sys.argv[1], sys.argv[2]
for f in FILES:
    a, b = (f, os.path.join(d, f)) if mode == 'save' else (os.path.join(d, f), f)
    if os.path.exists(a):
        os.makedirs(os.path.dirname(b) or '.', exist_ok=True)
        shutil.copyfile(a, b)
print(mode, d)
