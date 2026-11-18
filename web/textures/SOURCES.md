# Where the page's textures come from

Every image here is CC0 (no rights reserved; free for any use) from Poly Haven (polyhaven.com): the ground photographs
resized and packed for the page by hand, each with its large-scale light divided out (so its repeat does not show); the
carpets and the sprites rendered from Poly Haven's 3D models by the tool in `bake/`. Each file is a strip of 512 x 512
layers, top first.

## ground.jpg

| layer | Poly Haven asset | its side in the world |
|---|---|---|
| 0 | dry_river_pebbles (diffuse) | 2.0 m |
| 1 | aerial_rocks_02 (diffuse) | 50 m |
| 2 | brown_mud (diffuse) | 1.3 m |
| 3 | forest_leaves_02 (diffuse) | 3.0 m |

## carpets.jpg

Tiles of ground cover rendered with three.js from Poly Haven's CC0 3D models (glTF, 1k textures) scattered over a ground
texture: the ground from straight above, each thing standing up half its height up the screen (the map's angled view),
lit from the north-west with shadows, each thing repeated across the tile's edges so the tile repeats; rendered at
1024 x 1024 and halved. The tool is in `bake/`: `fetch.sh assets` downloads what `carpets.json` names; with a folder
served at localhost:8835 holding `assets/` and `render/` (the files of `bake/` beside three.js 0.160 in `node_modules`),
`node bake.cjs carpets.json out` (with Playwright) renders each tile to `out/<name>.png`, and the strip is the tiles in the order of the
table below.

| layer | ground | models | side |
|---|---|---|---|
| 0 | leafy_grass | grass_medium_01, grass_medium_02, weed_plant_02, celandine_01, dandelion_01 | 1.5 m |
| 1 | dry_ground_01 | grass_medium_01 and grass_medium_02 dried in colour, namaqualand_stones_01 | 1.5 m |
| 2 | forest_leaves_02 | dry_branches_medium_01, bark_debris_01, fern_02, periwinkle_plant, celandine_01, stone_01 | 3 m |
| 3 | leaves_forest_ground | fern_02, periwinkle_plant, celandine_01, grass_medium_02, dry_branches_medium_01, bark_debris_01 | 3 m |
| 4 | dry_ground_rocks | namaqualand_stones_01, stone_01, dry grass_medium_01, dead_quiver_branch_01, weed_plant_02 | 3 m |
| 5 | mud_cracked_dry_03 | namaqualand_stones_01, dry grass_medium_01, dead_quiver_branch_01 | 3 m |
| 6 | coast_sand_01 | namaqualand_stones_01, stone_01, dry grass_medium_01 and _02, dead_quiver_branch_01, bark_debris_01, weed_plant_02 | 3 m |
| 7 | white ground with wind ripples (sums of waves) | none | 3 m |
| 8 | brown_mud | grass_medium_01 stretched twice as tall, some dried in colour, grass_medium_02 | 2 m |
| 9 | none (black) | flower_ursinia, dandelion_01: their petals alone, grey, all else black | 0.8 m |
| 10 | leafy_grass | short turf: grass_medium_02 and _01 small and dense, weed_plant_02, celandine_01 | 2 m |
| 11 | forest_ground_04 | tussocks: grass_medium_01 and _02 large, in clumps, with bare ground and stones between | 2 m |
| 12 | dry_ground_01 | layer 10 dried in colour | 2 m |
| 13 | dry_ground_rocks | layer 11 dried in colour | 2 m |

grass_medium_01's tall stems with seed heads (its meshes named `_ta…`) are left out of every tile.

## trees.png, models.png and the sprites' table

Sprites rendered from Poly Haven's CC0 3D models (glTF, 1k textures) with three.js by `bake/sprite.html`: an
orthographic camera looking north and 35 degrees down, lit by the sky and by the sun from the north-west, matte, no
shadows, on a clear background; each sprite fills its cell but for a 64th of its side, its foot two texels above the
cell's bottom middle, rendered at twice the cell's side and shrunk. `trees.png` holds the trees one to a 512 x 512
layer; `models.png` the rest four to a layer. `bake/sprites.json` names each model, its parts (its nodes), the sides it
is seen from (turns about the vertical), how much of its foot is cut off (the patch of ground some trees stand on), and
whether it is set down on the face it rests on (the face of its convex hull nearest its centre of weight: the rocks)
and how far it is sunk into the ground (a fifth of its height, cut off at the ground line), which of its materials are
its leaves (a tree that sheds gets a bare twin: rendered again in the same frame with its leaves hidden, at half the
side, four to a layer after the models' own sprites), and the mean lightness its
leaves are scaled to (texels whose green stands out, as the page tells leaves: the models'
leaves differ several times in lightness as the files have them), and the sets the page draws from. With the files
fetched (`fetch.sh assets` also downloads what `sprites.json` names) and served as for the carpets,
`node sprites.cjs sprites.json out` writes `out/trees.png`, `out/models.png` and
`out/sprites.wgsl`, the table the page reads (`web/shaders/lib/sprites.wgsl`: each set's first sprite and count, each
sprite's width, height and mean cover); `node sprites.cjs sprites.json out <model>...` writes those models' sprites
alone, to compare.

| strip | sprites (sets) |
|---|---|
| trees.png | island_tree_01, island_tree_02 (broad crowns), jacaranda_tree, each from three sides (each with a bare twin); fir_tree_01's three trees each from three sides; pine_tree_01's three pines, pine_sapling_medium's two (young pines), pine_sapling_small's three, fir_sapling_medium's three (young spruces); quiver_tree_01 (with bare twins: its trunk and branches) and quiver_tree_02 (one material: no twin) each from two sides |
| models.png | shrub_02's four shrubs; rocks: boulder_01 from three sides, namaqualand_boulder_02, _03 and _05 each from two; wood: dead_tree_trunk, dead_tree_trunk_02 from two sides; dead bushes: wild_rooibos_bush's first four; grass tufts: grass_medium_01's meshes 0, 10, 1, 6, 7, 9 and grass_medium_02's 2, 4; bushes: searsia_lucida's 0, 2, 3, 4, 5, othonna_cerarioides's 0, 1, 2, 4, 5 and didelta_spinosa's largest; soft tall plants (palms, tree ferns, giant herbs), built: fern_02's clumps c, a and d and calathea_orbifolia_01's first, each set on a plain tapering stem of a tree fern's dark brown (1.2 and 0.8 times its height), and pachira_aquatica_01's first two trees (bark and leaves); then the trees' bare twins |
