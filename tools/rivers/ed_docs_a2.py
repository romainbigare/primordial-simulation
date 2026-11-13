def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('docs/tape.md', [
 ("""`tape/src/ground/rock.rs`), made once with the ground: of each of the terrain's samples, row by row, two 16-bit floats:
the depth of its loose ground, m (`Samples::loose`), and the largest stone its channel moves, m: `STONE_M` (0.001) ×
√(the area it drains, m²) × its slope to the sample it runs to (the stream power the wear reads, K √A S; Bagnold 1966),
at most 2 m; 0 for a sink or a sample under a pond. **The""",
  """`tape/src/ground/rock.rs`), made once with the ground: of each of the terrain's samples, row by row, four 16-bit floats:
the depth of its loose ground, m (`Samples::loose`), the largest stone its channel moves, m: `STONE_M` (0.001) ×
√(the area it drains, m²) × its slope to the sample it runs to (the stream power the wear reads, K √A S; Bagnold 1966),
at most 2 m; 0 for a sink or a sample under a pond; a zero; and its channel's links as one whole number: the step to
the sample it runs to (1 to 8, its neighbours row by row; 8 more where it joins that one as a tributary; 0 for none)
and 17 times the step to its main neighbour upstream, by which a shell lets what runs through a channel grow evenly
from one sample's centre to the next, so its width has no step where two samples meet. Then of each sample again the
reflectance of its bed's make and a zero. **The"""),
])
edit('README.md', [
 ("""below a metre), the wetness index of the place and the channel nearest it, as 16-bit floats (10 bytes a texel, the""",
  """below a metre), the wetness index of the place and the channel nearest it, as 16-bit floats (16 bytes a texel, the"""),
 ("""most 56 on the graphics card (38 MB) and no copy""", """most 56 on the graphics card (61 MB) and no copy"""),
 ("""and none a mesh of rills), wider as its discharge grows, to the power 0.75 up to 8 times, so the network's order reads.
Close up a channel's bed is as wide as the channel law makes five times this month's discharge (the bankfull flood,
Leopold and Maddock 1953; Dunne and Leopold 1978), bare of plants; this month's water winds in it between bars of
washed gravel, its thread wandering across the bed over a few bed widths and splitting about a bar where it wanders
farthest.""", """and none a mesh of rills), wider as its discharge grows, to the power 0.75 up to 4 times, so the network's order reads.
Close up a channel's bed is as wide as the channel law makes five times this month's discharge (the bankfull flood,
Leopold and Maddock 1953; Dunne and Leopold 1978), bare of plants, and on gentle ground it swings in a meander belt
(bends of many sizes, some reaches nearly straight, here and there a loop: `tape/src/ground/belt.rs`). This month's
water fills most of it (at one place the water's width grows as its discharge to the power 0.26: Leopold and Maddock
1953): a pool against the outer bank of a bend over a bar of washed gravel on its inside, a wider riffle between
bends; what runs through it grows evenly from sample to sample, so it widens downstream and below each tributary
without a step."""),
])
