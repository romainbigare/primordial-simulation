# Places (km from the map's north-west corner) per seed: shore, lake, river, ridge, plain.
P1="shore:35.8:11.7 lake:27.9:22.2 river:16.1:9.5 ridge:24.8:13.9 plain:11.7:20.4"
P2="shore:37.1:19.1 lake:18.3:32.7 river:19.1:20.4 ridge:22.2:33.6 plain:11.7:24.8"
SCALES="1km:10 200m:2 50m:0.5 10m:0.1"
shots() { for pl in $1; do n=${pl%%:*}; r=${pl#*:}; for sc in $SCALES; do echo "$n-${sc%%:*}:$r:${sc#*:}"; done; done; }
