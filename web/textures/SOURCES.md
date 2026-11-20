# Where the page's textures come from

Every image here is CC0 (no rights reserved; free for any use) from Poly Haven (polyhaven.com) and Kenney (kenney.nl), but for the trees
rendered from PlantCatalog's models, which are CC BY 4.0 (credited below): the ground photographs resized and packed for
the page by hand, each with its large-scale light divided out (so its repeat does not show); the carpets and the sprites
rendered from 3D models by the tool in `bake/`. Each file is a strip of 512 x 512 layers, top first.

## ground.jpg

| layer | Poly Haven asset | its side in the world |
|---|---|---|
| 0 | dry_river_pebbles (diffuse) | 2.0 m |
| 1 | aerial_rocks_02 (diffuse) | 50 m |
| 2 | brown_mud (diffuse) | 1.3 m |
| 3 | forest_leaves_02 (diffuse) | 3.0 m |
| 4 | damp_beach_sand (diffuse) | 2.0 m |
| 5 | coast_sand_02 (diffuse) | 15 m |
| 6 | brown_mud_02 (diffuse) | 1.3 m |
| 7 | ganges_river_pebbles (diffuse) | 2.2 m |
| 8 | river_small_rocks (diffuse) | 2.9 m |
| 9 | brown_mud_leaves_01 (diffuse) | 1.3 m |
| 10 | forest_leaves_03 (diffuse) | 2.3 m |
| 11 | dirt_floor (diffuse), https://polyhaven.com/a/dirt_floor | 2.07 m |
| 12 | mud_cracked_dry_riverbed_002 (diffuse), https://polyhaven.com/a/mud_cracked_dry_riverbed_002 | 2.0 m |
| 13 | mud_forest (diffuse), https://polyhaven.com/a/mud_forest | 2.35 m |

Each is the 2k diffuse shrunk to 512 x 512, its light divided by itself blurred (a Gaussian of 40 texels, wrapping
round, so the layer still repeats) and scaled back to its mean. All are CC0 (polyhaven.com/license).

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

## fire.jpg

Two 512 x 512 layers, made by `bake/fire.py` (`python3 -I bake/fire.py KENNEY_PNG_DIR burned_ground_01_diff_2k.jpg
fire.jpg`):

| layer | what | source | author | licence |
|---|---|---|---|---|
| 0 | four sprites, a quarter each, their density alone (grey times cover) on black, each fitted to its quarter with its foot on the bottom middle: two flame tongues (`flame_05`, `flame_06`), two puffs of smoke (`smoke_07`, `smoke_08`) | [Particle Pack 1.1](https://kenney.nl/assets/particle-pack), its `PNG (Transparent)` folder | Kenney Vleugels (kenney.nl) | CC0 1.0 |
| 1 | burnt ground: charred soil, ash, needles and grass stalks; 1 m across | [burned_ground_01](https://polyhaven.com/a/burned_ground_01) (diffuse, 2k), shrunk, its light divided out as `ground.jpg`'s | Rob Tuytel (Poly Haven) | CC0 1.0 |

## snags.png

Two 512 x 512 layers of four quarters, each a sprite with its cover, rendered by `bake/sprite.html` as the trees are
(`node sprites.cjs snags.json OUT burned_tree_photogrammetry dead_trees burned_trunk_lassen`, the models downloaded by
hand into `assets/sketchfab/<the model's id>/`) and packed by `bake/snags.py` (`python3 -I bake/snags.py OUT
snags.png`): three charred standing trunks (one model from three sides), a stand of dead stems, and three charred
stumps (one model from three sides, cut 0.3 of its height above its foot, so the ground it was scanned on is left
out). Each licensed under CC BY 4.0 (http://creativecommons.org/licenses/by/4.0/); the sprites are rendered from them
(changed: lit, cut at the ground, shrunk). Not yet drawn by the page.

| sprite | model | author | licence |
|---|---|---|---|
| `burned_tree_photogrammetry` | [Burned Tree Photogrammetry](https://sketchfab.com/3d-models/burned-tree-photogrammetry-1dc8bcac00e2433eb645e6c837a8d5b7) | [gino.demurtas](https://sketchfab.com/gino.demurtas) | CC BY 4.0 |
| `dead_trees` | [Dead Trees](https://sketchfab.com/3d-models/dead-trees-41f9d44a8d8b45549dd6a0c947ca4eba) | [Darklordbroch](https://sketchfab.com/Darklordbroch) | CC BY 4.0 |
| `burned_trunk_lassen` | [Burned tree trunk - Lassen Volcano Park](https://sketchfab.com/3d-models/burned-tree-trunk-lassen-volcano-park-82f510d9e012414d9aa980bf944efc93) | [David Antalek](https://sketchfab.com/David.Antalek) | CC BY 4.0 |

## trees.png, models.png and the sprites' table

Sprites rendered from Poly Haven's CC0 3D models (glTF, 1k textures) and from Sketchfab's CC0 and CC BY 4.0 models
(PlantCatalog's and the water's edge models below: glTF, or FBX at its finest level of detail) with three.js by `bake/sprite.html`: an
orthographic camera looking north and 35 degrees down, lit by the sky and by the sun from the north-west, matte, no
shadows, on a clear background; each sprite fills its cell but for a 64th of its side, its foot two texels above the
cell's bottom middle, rendered at twice the cell's side and shrunk. `trees.png` and `trees2.png` hold the trees one to
a 512 x 512 layer; `models.png` the rest four to a layer. `bake/sprites.json` names each model, its parts (its nodes), the sides it
is seen from (turns about the vertical), how much of its foot is cut off (the patch of ground some trees stand on), and
whether it is set down on the face it rests on (the face of its convex hull nearest its centre of weight: the rocks)
and how far it is sunk into the ground (a fifth of its height, cut off at the ground line), whether it is stood as a
clump (the reeds, cattails and sedges: a scanned stem stood several times about one foot, each copy turned, shrunk to
between 0.6 and its whole size and set off from the foot, by a fixed draw), which of its materials are
its leaves (a tree that sheds gets a bare twin: rendered again in the same frame with its leaves hidden, at half the
side, four to a layer after the models' own sprites), and the mean lightness its
leaves are scaled to (texels whose green stands out, as the page tells leaves: the models'
leaves differ several times in lightness as the files have them), and the sets the page draws from; a model with a
`file` is read from that file under `assets/` (the Sketchfab models, downloaded by hand from the links below into
`assets/sketchfab/<the model's id>/`: a glTF model's `scene.gltf`, an FBX model's finest level, `source/..._LOD0.fbx`,
each with its textures). The models that stand in water (`height`: the bald cypress, the mangrove, the reeds, cattails and
rushes) are rendered once more in the same frame with each texel's height over the ground as a share of the sprite's
height for its grey (held to 1), at half a tree's side, four to a 512 x 512 layer of `heights.png`; with them the
bake writes `heights.wgsl` (`web/shaders/lib/heights.wgsl`): which sprites have heights, and of each the height of its
stem's foot (the middle of its model's box on the ground) over its lowest point. The mangrove is cut at 0.09 of its
height: its prop roots end in the ground there, and only a few root tips reach lower. With the files fetched (`fetch.sh assets` also downloads the Poly Haven models `sprites.json`
names) and served as for the carpets, `node sprites.cjs sprites.json out` writes the strips, `out/models.png` and
`out/trees.png`, each split into more files of whole layers (`trees2.png`, ...) where one would be over 12 MB (the bake
tool's `STRIP_BYTES`: the owner's page host takes no file over 15 MB), `out/strips.json`, the strips' files in their
layers' order (`web/textures/strips.json`, which the page is built with), and `out/sprites.wgsl`, the table the page reads (`web/shaders/lib/sprites.wgsl`: each set's first sprite and count, each
sprite's width, height, mean cover and mean leaf lightness) with the heights' strip and table; `node sprites.cjs
sprites.json out <model>...` writes those models' sprites alone, to compare; `node sprites.cjs sprites.json out
--heights` the heights alone.

| strip | sprites (sets) |
|---|---|
| trees.png, trees2.png | round: island_tree_01, island_tree_02 each from three sides, northern_red_oak_5 from two, red_horse_chestnut_16 and _24, dwarf_red_horse_chestnut_17; umbrella: jacaranda_tree from three sides, african_continental_baobab_1, _15 and _5; quiver: quiver_tree_01 and joshua_tree_19 each from two sides; lobed: black_poplar_50, common_fig_tree_11 and _30 each from two sides; column: lombardy_poplar_55 from two sides, black_poplar_25; conifers: norway_spruce_17, _18 and _23, engelmann_spruce_17 and _3, black_spruce_10, mountain_hemlock_21 and _23, fir_tree_01's first two trees; young spruces: fir_sapling_medium's three, black_spruce_16 from two sides; pines: pine_tree_01's three; young pines: pine_sapling_medium's two and pine_sapling_small's three; palms: date_palm_16 and _28, california_fan_palm_22, mexican_fan_palm_3, canary_island_date_palm_21, mediterranean_fan_palm_19; ferns: male_fern_20. The broad-leaved trees, the baobabs and quiver_tree_01 have bare twins (their leaves hidden); cypresses (flared feet): great_bald_cypress from two sides; mangroves (propped feet): red_mangrove from two sides |
| models.png | shrub_02's four shrubs; rocks: boulder_01 from three sides, namaqualand_boulder_02, _03 and _05 each from two; wood: dead_tree_trunk, dead_tree_trunk_02 from two sides; dead bushes: wild_rooibos_bush's first four; grass tufts: grass_medium_01's meshes 0, 10, 1, 6, 7, 9 and grass_medium_02's 2, 4; bushes: searsia_lucida's 0, 2, 3, 4, 5, othonna_cerarioides's 0, 1, 2, 4, 5 and didelta_spinosa's largest; shore rocks: rock_moss_set_02's meshes 0, 1, 2, 3, 4, 6, rock_moss_set_01's 0 and 2, rock_09, namaqualand_boulder_04 and _06; driftwood: dead_quiver_branch_01 and _02, dry_branches_medium_01's three, pine_fallen_trunk_02 and _01 and mossy_old_log each from two sides; reeds: reed_plants's five stems each stood as a clump of 9, 12, 7, 10 and 8; cattails: cattail's stem stood as clumps of 6 and 11, cattail_clusters from one side; rushes: cattail's two leaf tufts (meshes 11, 14) each stood 4 times, carex_dispalata's two stems each stood 8 times, carex_maximowiczii's stem stood 14 times; pads: water_lily's floating leaf (meshes 33 to 37) from three sides, cut at its stalk; a pad's flower: water_lily's flower (meshes 6 to 23) cut at its stalk; patches of pads: lily_pads's mesh 2 from two sides, cut at its stalks; cobbles: rocks_pack_5's meshes 4, 6, 8, 10, 12, 16, 18, 22, 32, 34, 40, 42, rock_scan_11 and rock_scan_12; then the trees' bare twins |

The trees from PlantCatalog's models (author PlantCatalog, https://sketchfab.com/PlantCatalog), each licensed under CC
BY 4.0 (http://creativecommons.org/licenses/by/4.0/); the sprites are rendered from them (changed: lit, cut at the
ground, shrunk, their leaves' lightness scaled):

| sprite | model |
|---|---|
| `northern_red_oak_5` | [Realistic HD Northern red oak (5/138)](https://sketchfab.com/3d-models/realistic-hd-northern-red-oak-5138-06474083fabe4e0f88e40848ea7aa8f9) |
| `red_horse_chestnut_16` | [Realistic HD Red horse chestnut (16/30)](https://sketchfab.com/3d-models/realistic-hd-red-horse-chestnut-1630-bc56ed3eb15948c6b61c75eff557eac7) |
| `red_horse_chestnut_24` | [Realistic HD Red horse chestnut (24/30)](https://sketchfab.com/3d-models/realistic-hd-red-horse-chestnut-2430-578463dffe1e45cbb99c529886ca17c7) |
| `dwarf_red_horse_chestnut_17` | [Realistic HD Dwarf red horse chestnut (17/20)](https://sketchfab.com/3d-models/realistic-hd-dwarf-red-horse-chestnut-1720-0f38b4d5eb934fa49829a841c68ee7a5) |
| `african_continental_baobab_1` | [Realistic HD African continental baobab (1/20)](https://sketchfab.com/3d-models/realistic-hd-african-continental-baobab-120-d42e970e89ce43f9b06cfef4d466eb7f) |
| `african_continental_baobab_15` | [Realistic HD African continental baobab (15/20)](https://sketchfab.com/3d-models/realistic-hd-african-continental-baobab-1520-f76b402dbdfb4d4b873fba9ea38f507d) |
| `african_continental_baobab_5` | [Realistic HD African continental baobab (5/20)](https://sketchfab.com/3d-models/realistic-hd-african-continental-baobab-520-d3c3363a23704d92a4368974cad63c5a) |
| `joshua_tree_19` | [Realistic HD Joshua tree (19/30)](https://sketchfab.com/3d-models/realistic-hd-joshua-tree-1930-7198687a6a9645e38e33108cbb4c71dc) |
| `black_poplar_50` | [Realistic HD Black poplar (50/105)](https://sketchfab.com/3d-models/realistic-hd-black-poplar-50105-562a96bc020e49c68f1f001dbe585cab) |
| `common_fig_tree_11` | [Realistic HD Common fig tree (11/30)](https://sketchfab.com/3d-models/realistic-hd-common-fig-tree-1130-bcbde4dd01264c6e82954f9acc9fe40e) |
| `common_fig_tree_30` | [Realistic HD Common fig tree (30/30)](https://sketchfab.com/3d-models/realistic-hd-common-fig-tree-3030-2e6b64be3c6a466eb6b86591d9ef2ebc) |
| `lombardy_poplar_55` | [Realistic HD Lombardy poplar (55/75)](https://sketchfab.com/3d-models/realistic-hd-lombardy-poplar-5575-1884f92a179a408ca472e6a2f6469497) |
| `black_poplar_25` | [Realistic HD Black poplar (25/105)](https://sketchfab.com/3d-models/realistic-hd-black-poplar-25105-a927d7f2dd6a41fa855e93dcfd6fdb96) |
| `norway_spruce_17` | [Realistic HD Norway spruce (17/43)](https://sketchfab.com/3d-models/realistic-hd-norway-spruce-1743-652321a0e80c4bb3a36dbe6c9542e3cb) |
| `norway_spruce_18` | [Realistic HD Norway spruce (18/43)](https://sketchfab.com/3d-models/realistic-hd-norway-spruce-1843-9391e9fda97f4efa98e075442291a17b) |
| `norway_spruce_23` | [Realistic HD Norway spruce (23/43)](https://sketchfab.com/3d-models/realistic-hd-norway-spruce-2343-facebfa5baf84c4aafc1141745570714) |
| `engelmann_spruce_17` | [Realistic HD Engelmann spruce (17/43)](https://sketchfab.com/3d-models/realistic-hd-engelmann-spruce-1743-e9b6fb45fb1b4227982231f62d00684e) |
| `engelmann_spruce_3` | [Realistic HD Engelmann spruce (3/43)](https://sketchfab.com/3d-models/realistic-hd-engelmann-spruce-343-70e6fc24f73146a0866a87fccd168076) |
| `black_spruce_10` | [Realistic HD Black spruce (10/38)](https://sketchfab.com/3d-models/realistic-hd-black-spruce-1038-32e73f7c100e44768370b12bff737d34) |
| `mountain_hemlock_21` | [Realistic HD Mountain hemlock (21/43)](https://sketchfab.com/3d-models/realistic-hd-mountain-hemlock-2143-225cd2dfbf7d4163bc3a062a0c0623aa) |
| `mountain_hemlock_23` | [Realistic HD Mountain hemlock (23/43)](https://sketchfab.com/3d-models/realistic-hd-mountain-hemlock-2343-9b77f3c62f764602a76efae67704eb71) |
| `black_spruce_16` | [Realistic HD Black spruce (16/38)](https://sketchfab.com/3d-models/realistic-hd-black-spruce-1638-a9d79680963d44c3acf025e172c5bbec) |
| `date_palm_16` | [Realistic HD Date palm (16/78)](https://sketchfab.com/3d-models/realistic-hd-date-palm-1678-2f34e3f54bef40eb8d94ca6507c2f822) |
| `date_palm_28` | [Realistic HD Date palm (28/78)](https://sketchfab.com/3d-models/realistic-hd-date-palm-2878-52c8a8dc801c4c3492992e0a3f32efc1) |
| `california_fan_palm_22` | [Realistic HD California fan palm (22/25)](https://sketchfab.com/3d-models/realistic-hd-california-fan-palm-2225-5492bd27db734608bb41409efeba238c) |
| `mexican_fan_palm_3` | [Realistic HD Mexican fan palm (3/10)](https://sketchfab.com/3d-models/realistic-hd-mexican-fan-palm-310-5591dc5028ef466db828a962d4780a87) |
| `canary_island_date_palm_21` | [Realistic HD Canary Island date palm (21/40)](https://sketchfab.com/3d-models/realistic-hd-canary-island-date-palm-2140-7eac1bcdfe984bf8ba3fcd2430854a67) |
| `mediterranean_fan_palm_19` | [Realistic HD Mediterranean fan palm (19/30)](https://sketchfab.com/3d-models/realistic-hd-mediterranean-fan-palm-1930-db1b8aa7f06b499f9de68460de1eea6b) |
| `male_fern_20` | [Realistic HD Male fern (20/50)](https://sketchfab.com/3d-models/realistic-hd-male-fern-2050-9da5a089892d480fa3d2add186ce4f03) |

The water's edge and the swamp trees from these Sketchfab models, each licensed under CC BY 4.0
(http://creativecommons.org/licenses/by/4.0/) or given to the public domain under CC0 1.0
(http://creativecommons.org/publicdomain/zero/1.0/); the sprites are rendered from them (changed: lit, cut at the ground,
shrunk, stood as clumps, their leaves' lightness scaled):

| sprite | model | author | licence |
|---|---|---|---|
| `reed_plants` | [Reed Plants Pack](https://sketchfab.com/3d-models/reed-plants-pack-bc77ea5218e44f2dbc23b2a4aef0c898) | [Nicholas-3D](https://sketchfab.com/Nicholas01) | CC BY 4.0 |
| `cattail` | [Рогоз (Cattail)](https://sketchfab.com/3d-models/cattail-00cf5598ba274050b784b8a4beb00f03) | [Hakujin](https://sketchfab.com/hakujin) | CC BY 4.0 |
| `cattail_clusters` | [Cattail Clusters](https://sketchfab.com/3d-models/cattail-clusters-7a37d2dafc88414a8cb22eca27b1414c) | [kirbinator505_](https://sketchfab.com/kirbinator505_) | CC BY 4.0 |
| `carex_dispalata` | [CC0 カサスゲ Kasa-sge Sedge, Carex dispalata](https://sketchfab.com/3d-models/cc0-kasa-sge-sedge-carex-dispalata-79f4b32c824c45be8e3f668d18062cdf) | [ffish.asia / floraZia.com](https://sketchfab.com/ffishAsia-and-floraZia) | CC0 1.0 |
| `carex_maximowiczii` | [CC0 ゴウソ Maximowicz's Sedge, Carex maximowiczii](https://sketchfab.com/3d-models/cc0-maximowiczs-sedge-carex-maximowiczii-5737d74736ab440cb5b87f1c23ff96ff) | [ffish.asia / floraZia.com](https://sketchfab.com/ffishAsia-and-floraZia) | CC0 1.0 |
| `water_lily` | [CC0 スイレン Water Lily, Nymphaea cv.](https://sketchfab.com/3d-models/cc0-water-lily-nymphaea-cv-e7830f486a0248b6b894f3e7a63b8c16) | [ffish.asia / floraZia.com](https://sketchfab.com/ffishAsia-and-floraZia) | CC0 1.0 |
| `lily_pads` | [Lily Pads](https://sketchfab.com/3d-models/lily-pads-a732c7e7de0d43b8bf498b53fc163831) | [kirbinator505_](https://sketchfab.com/kirbinator505_) | CC BY 4.0 |
| `rocks_pack_5` | [Rocks - Pack 5](https://sketchfab.com/3d-models/rocks-pack-5-3b55666b3b8e41a3a72d94a1d5a76072) | [Loïc Norgeot](https://sketchfab.com/norgeotloic) | CC BY 4.0 |
| `rock_scan_11` | [Rock Scan - 11](https://sketchfab.com/3d-models/rock-scan-11-6195afc03d5f4c539273aa24873ef808) | [Loïc Norgeot](https://sketchfab.com/norgeotloic) | CC BY 4.0 |
| `rock_scan_12` | [Rock Scan - 12](https://sketchfab.com/3d-models/rock-scan-12-462ba9c87ce1445191899a6b74f4203c) | [Loïc Norgeot](https://sketchfab.com/norgeotloic) | CC BY 4.0 |
| `pine_fallen_trunk_02` | [Pine Tree Fallen Trunk Scan 02](https://sketchfab.com/3d-models/pine-tree-fallen-trunk-scan-02-f5ad948275cd4d13ba3ce2040b72ed47) | [Per's Scan Collection](https://sketchfab.com/perz_scans) | CC BY 4.0 |
| `pine_fallen_trunk_01` | [Pine Tree Fallen Trunk Scan 01](https://sketchfab.com/3d-models/pine-tree-fallen-trunk-scan-01-eca72dd7e85941eb8ecb9907df8aa560) | [Per's Scan Collection](https://sketchfab.com/perz_scans) | CC BY 4.0 |
| `mossy_old_log` | [Mossy old tree log](https://sketchfab.com/3d-models/mossy-old-tree-log-c65a00ca4a174653beb4c59cb42b9143) | [Julian Malik](https://sketchfab.com/julianmalik) | CC BY 4.0 |
| `great_bald_cypress` | [Great Bald Cypress](https://sketchfab.com/3d-models/great-bald-cypress-dca97787b1b24fd8a0fb6bb536e383e3) | [BaptisteBerard](https://sketchfab.com/BaptisteBerard) | CC BY 4.0 |
| `red_mangrove` | [red-mangrove-tree](https://sketchfab.com/3d-models/red-mangrove-tree-e806105d83bc440499495e62bb010f7a) | [ingdearco](https://sketchfab.com/ingdearco), published by [SAVIA_Colombia](https://sketchfab.com/Minambiente_Col) | CC BY 4.0 |

