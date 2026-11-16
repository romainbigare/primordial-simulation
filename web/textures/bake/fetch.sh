#!/bin/bash
# fetch.sh DIR: downloads into DIR/gltf/<name>/ the Poly Haven CC0 models (glTF, 1k) and into DIR/ the ground textures
# (diffuse, 2k) that carpets.json names.
set -e
cd "$1"
for n in $(python3 -c "import json;d=json.load(open('$(dirname "$0")/carpets.json'));print(' '.join(sorted({i['model'] for s in d.values() for i in s['items']})))"); do
  [ -f gltf/$n/$n.gltf ] && continue
  mkdir -p gltf/$n
  curl -sS https://api.polyhaven.com/files/$n | python3 -c "
import json,sys,os,urllib.request
d=json.load(sys.stdin)['gltf']['1k']['gltf']; n='$n'
urllib.request.urlretrieve(d['url'], f'gltf/{n}/{n}.gltf')
for k,v in d['include'].items():
  os.makedirs(os.path.dirname(f'gltf/{n}/{k}') or '.', exist_ok=True); urllib.request.urlretrieve(v['url'], f'gltf/{n}/{k}')"
done
for t in $(python3 -c "import json;d=json.load(open('$(dirname "$0")/carpets.json'));print(' '.join(sorted({s['ground']['tex'][:-4] for s in d.values() if 'tex' in s['ground']})))"); do
  [ -f $t.jpg ] && continue
  curl -sS -o $t.jpg "$(curl -sS https://api.polyhaven.com/files/$t | python3 -c "import json,sys;print(json.load(sys.stdin)['Diffuse']['2k']['jpg']['url'])")"
done
