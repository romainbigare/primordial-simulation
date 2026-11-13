#!/bin/bash
# Commit round four's work in pieces.
cd /home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520 || exit 1
T="
Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01QN7cBuSoEzcgd4DH5ugPyk"
git add web/shaders/lib/flow.wgsl
git commit -q -m "White water begins softly below a step: no lattice of the tiles' texels

The place along a channel is kept as a 16-bit float, a few centimetres apart; a
step's white water began over two pixels, sharper than that, and drew the
texels' lattice in fast water close up.
$T"
git add tape/src/ground/valley.rs tape/src/ground/mend.rs tape/src/ground/channels.rs
git commit -q -m "Valleys without steps; strips two texels wide mended

A valley's floor at a point is the weighted mean of the line's floors about it,
piece by piece, not the floor of the nearest point: that one leapt where a line
bends, and drew thin straight steps across the ground (the dark strokes at the
200 m bar). A small channel's strip across a larger one's reach is mended up
to two texels wide.
$T"
git add tape/src/ground/wander.rs tape/src/ground/bends.rs tape/src/ground/stems.rs tape/src/ground/lines.rs
git commit -q -m "Every channel wanders: rills, and a river's last reach to its mouth

The wander has five octaves; a channel takes those whose corners lie eight of
its bed's widths apart or more, so a rill turns every few metres and a wide bed
keeps its bends. A stem's points are smoothed through a body's samples, and the
line wanders there too. The power a channel's water widens by at one place
lives beside the flood's number.
$T"
git add -A
git commit -q -m "One rule for a mouth's bed and fan; snow bridges a rill; less work a pixel

A channel's widening at its mouth is part of the channel itself, and its fan is
one function: the land draws them and a plant's foot reads them, so no plant
roots in the widened bed or on the fan. Snow deeper than a channel's water is
wide hides it. A channel's law is read once a pixel, its neighbours and stones
only where they can show, and the share of its bed its water takes comes with
the uniforms.
$T"
git log --oneline | head -5
