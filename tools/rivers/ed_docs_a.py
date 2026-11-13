def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('docs/tape.md', [
 ("""texel's channel, m, signed by its side), then two (that channel's sample, and the pond), 10 bytes a texel. The depths""",
  """texel's channel, m, signed by its side), then four (that channel's sample, the pond, and of the nearest point of that
channel's line how far along the channel it lies and how the line curves there), 16 bytes a texel. The depths"""),
 ("""(`tape/src/ground/stems.rs`): the mean of the centres of the samples along its stem, 3 up and 3 down, weighted 3, 2, 1
by nearness and the sample's own 4 (a moving average of the polyline of centres, as corner cutting smooths a polygon:
Chaikin 1974), as many each way as the stem has, so a head, the end of a stem at a body and the sample a tributary
joins keep their place (a tributary ends on the joined stem's point), and no more than half a sample from the centre:
so a channel that steps like a stair down a valley across the grid draws as one smooth line.""",
  """(`tape/src/ground/stems.rs`): the mean of the centres of the samples along its stem, 6 up and 6 down, weighted 6 to 1
by nearness and the sample's own 7 (a moving average of the polyline of centres, as corner cutting smooths a polygon:
Chaikin 1974), as many each way as the stem has, so a head, the end of a stem at a body and the sample a tributary
joins keep their place (a tributary ends on the joined stem's point), and no more than a sample from the centre:
so a channel that steps like a stair down a valley across the grid draws as one smooth line from afar."""),
 ("""ground's liquid. The line winds about that curve (`tape/src/ground/bends.rs`): a sine whose wavelength is 11 channel
widths and whose half amplitude 1.35 widths (Leopold and Wolman 1960), full where the valley's slope to the next sample
is under 0.01, none over 0.03 (pool-riffle reaches meander, step-pool ones do not: Montgomery and Buffington 1997;
Schumm 1985), and no wider than the valley's floor: the channel's depth over the rise of the ground across the valley
(the two samples square to its way). A tile is made once, so the width and the depth that set the bends are stand-ins:
the channel law's at the drained area times `TYPICAL_RUNOFF` (8.5e-9 m/s, Earth's mean: 0.27 m a year, Dai and
Trenberth 2002). The phase of the bends runs on along each main way from its head, so the line has no break from
sample to sample; at a joint both pieces take the smaller amplitude. A tile draws the bends where their wavelength is
8 texels or more, and none under 4 (between they grow): shorter, they move the line by less than a texel. Each""",
  """ground's liquid. On gentle ground the line swings about that curve in a meander belt (`tape/src/ground/bends.rs`,
`belt.rs`). A bed is shaped by its floods, so the bends are sized by the bed's width; a tile is made once, so that
width and the bed's depth are stand-ins: the channel law's at the drained area times `BED_RUNOFF`, 5 times (`FLOOD`: the
bankfull flood, Leopold and Maddock 1953; Dunne and Leopold 1978) `TYPICAL_RUNOFF` (8.5e-9 m/s, Earth's mean: 0.27 m a
year, Dai and Trenberth 2002). The belt reaches 8 bed widths to each side where nothing holds it (belts are 4 to 20
widths across, Williams 1986): all of it where the reach's slope (over 2 samples up and 2 down the channel) is under
0.01, none over 0.03 (pool-riffle reaches meander, step-pool ones do not: Montgomery and Buffington 1997; Schumm 1985);
no wider than the valley's floor (4 bed depths over the rise of the ground across the valley, the samples square to
the channel's way over the same reach), than 0.4 of the radius the valley's own line turns on, and than 0.8 of a sample;
none for a bed under 1 m, under standing liquid or at a sink. The line is walked down each main way from its head, from
one apex to the next (a half bend, 24 steps): along a half bend the curvature runs smoothly from the first apex's to the
next one's turned the other way (a cubic of the heading, 0 at both apexes: a curvature that falls evenly along the line
where both are alike, as measured bends do: Langbein and Leopold 1966), so the line has no kink and its curvature no
jump. Each apex takes its place and its radius from the place's draws (a function of the seed, the sample the bends
began at and the half bends since): the belt's middle wanders from the valley's line by up to half the belt's half
width over 7 half bends; an apex reaches 0.5 to 1 of the rest from it, times a swell that falls to 0.1 over 5 half
bends (most reaches bend fully, some run nearly straight), on the other side from the last; it turns on a radius of
1.5 to 3.5 bed widths, the tighter the more often (2 to 3 in most grown bends: Hickin and Nanson 1975), gentler where
the swell is low. A half bend is as long as carries the line from its apex to the next (found by halving, 3 to 24
widths), no longer than keeps its heading within 1.85 rad of the valley's line and carries it 2.5 widths down the
valley: past a right angle a bend turns back on itself in a loop, and two limbs of a loop come no nearer than about a
bed's width. A piece's walk starts as the last one's ended, so the line has no break from sample to sample, and along
it runs the line's own length from its head; a tributary's and a sink's last piece draw back to their curve at their
end. A tile draws the bends where their wavelength (11 bed widths, Leopold and Wolman 1960) is 24 texels or more, and
none under 12 (between they grow): the bed is then a texel or two wide, and from farther a shell draws it as a line
wider than its bends. Each"""),
 ("""narrower than a texel, the nearest), but a tributary takes no texel in the water of the channel it joins (within that
channel's half width at 10 × `TYPICAL_RUNOFF` of its line, near the join). It keeps that channel's distance, signed by
the side (60,000 where none reaches), and its sample, as the step from the sample that holds the texel's centre
(1 + (dx + 21) + 43 (dy + 21); 0 for none), whole numbers a 16-bit float holds exactly.""",
  """narrower than a texel, the nearest), but a tributary takes no texel in the bed of the channel it joins (within half
that bed's width of its line, near the join). It keeps that channel's distance, signed by
the side (60,000 where none reaches), and its sample, as the step from the sample that holds the texel's centre
(1 + (dx + 21) + 43 (dy + 21); 0 for none), whole numbers a 16-bit float holds exactly; and of the nearest point of
that channel's line, how far along the channel from its head it lies, m, modulo 128 m (`ALONG_WRAP_M`: a 16-bit float
holds it to a few centimetres, and a shell lays along it only signs that come again after that length), and the line's
curvature, 1/m, positive where it turns toward the side the distance is positive on: by these a shell draws the pool
at the outer bank of a bend, the bar inside it and the signs of the flow."""),
])
