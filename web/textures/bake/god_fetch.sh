#!/bin/bash
# god_fetch.sh DIR: downloads into DIR what god.json names from Poly Haven (all CC0): each texture source that is not a
# layer of the game's strips (its diffuse, 2k JPEG), the relief of those god.json lists under "relief" (2k PNG), and
# each sky panorama ("sky", its tone-mapped JPEG). Files already there are kept.
set -e
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$1"
cd "$1"
read -r textures reliefs skies < <(python3 -I -c "import json
d=json.load(open('$here/god.json'))
t=sorted({v[0] for p in d['pictures'].values() for v in p.get('textures', {}).values() if ':' not in v[0]})
s=sorted({p['sky'] for p in d['pictures'].values() if 'sky' in p})
print(','.join(t), ','.join(d['relief']), ','.join(s))")
url() { curl -sS "https://api.polyhaven.com/files/$1" | python3 -I -c "import json,sys;d=json.load(sys.stdin);print($2)"; }
for t in ${textures//,/ }; do
  [ -f "${t}_diff_2k.jpg" ] || curl -sS -o "${t}_diff_2k.jpg" "$(url "$t" "d['Diffuse']['2k']['jpg']['url']")"
done
for t in ${reliefs//,/ }; do
  [ -f "${t}_disp_2k.png" ] || curl -sS -o "${t}_disp_2k.png" "$(url "$t" "d['Displacement']['2k']['png']['url']")"
done
for s in ${skies//,/ }; do
  [ -f "$s.jpg" ] || curl -sS -o "$s.jpg" "$(url "$s" "d['tonemapped']['url']")"
done
