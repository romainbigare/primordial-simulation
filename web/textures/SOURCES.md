# Where the page's textures come from

Every image here is CC0 (no rights reserved; free for any use) from Poly Haven (polyhaven.com), resized and packed for the
page by hand, each ground photograph with its large-scale light divided out (so its repeat does not show). Each file is
a strip of 512 x 512 layers, top first.

## ground.jpg

| layer | Poly Haven asset | its side in the world |
|---|---|---|
| 0 | leaves_forest_ground (diffuse) | 1.26 m |
| 1 | forest_leaves_02 (diffuse) | 3.0 m |
| 2 | leafy_grass (diffuse) | 2.0 m |
| 3 | rocky_trail (diffuse) | 2.0 m |
| 4 | mud_cracked_dry_03 (diffuse) | 1.5 m |
| 5 | dry_river_pebbles (diffuse) | 2.0 m |
| 6 | coast_sand_01 (diffuse) | 15 m |
| 7 | aerial_rocks_02 (diffuse) | 50 m |
| 8 | brown_mud (diffuse) | 1.3 m |
| 9 | leaves cut from nettle_plant (diffuse and alpha), scattered into a tile | — |
| 10 | fronds cut from fern_02 (diffuse and alpha), scattered into a tile | — |
| 11 | bark_brown_02 (diffuse) | — |

## carpets.jpg

Tiles of ground cover rendered with three.js from Poly Haven's CC0 3D models (glTF, 1k textures) scattered over a ground
texture: the ground from straight above, each thing standing up half its height up the screen (the map's angled view),
lit from the north-west with shadows, each thing repeated across the tile's edges so the tile repeats; rendered at
1024 x 1024 and halved. The tool is in `bake/`: `fetch.sh assets` downloads what `carpets.json` names; with a folder
served at localhost:8835 holding `assets/` and `render/` (`bake.html` beside three.js 0.160 in `node_modules`),
`node bake.js carpets.json out` renders each tile to `out/<name>.png`, and the strip is the tiles in the order of the
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

## trees.png

The trees rendered as `models.png` below, at 1,024 x 1,024 and set one to a 512 x 512 layer, foot on the bottom middle:
island_tree_01, island_tree_02 and jacaranda_tree each from three sides with the patch of ground at their foot cut off
(the lowest 5% of their height), then fir_tree_01's three trees each from three sides; then, each from one side,
pine_tree_01's three pines, pine_sapling_medium's two, pine_sapling_small's three and fir_sapling_medium's three (the
lowest 2% cut off), and quiver_tree_01 and quiver_tree_02 each from two sides. Each new tree's colours are scaled so
its leaves are as light on average as the first trees' (their other texels by the square root of that).

## models.png

Sprites rendered from Poly Haven's CC0 3D models (glTF, 1k textures) with three.js, an orthographic camera looking
down 35 degrees, lit from the north-west; each model from three sides; four to a 512 x 512 layer, foot on the bottom
middle of its quarter.

| quarters | Poly Haven model |
|---|---|
| 0 to 3 | shrub_02 (its four shrubs) |
| 4 to 6 | boulder_01 |
| 7 to 9 | dead_tree_trunk |
| 10, 11 | namaqualand_boulder_02 (two sides) |
| 12, 13 | namaqualand_boulder_03 (two sides) |
| 14, 15 | namaqualand_boulder_05 (two sides) |
| 16, 17 | dead_tree_trunk_02 (two sides) |
| 18 to 21 | wild_rooibos_bush (four of its bushes) |
| 22 to 27 | grass_medium_01 (its meshes 0, 10, 1, 6, 7, 9 in file order) |
| 28, 29 | grass_medium_02 (its meshes 2, 4 in file order) |
| 30 to 34 | searsia_lucida (its bushes 0, 2, 3, 4, 5) |
| 35 to 39 | othonna_cerarioides (its bushes 0, 1, 2, 4, 5) |
| 40 | didelta_spinosa |

The bushes from 30 on are rendered at 1,024 x 1,024 and their leaves made as light on average as the trees'.
