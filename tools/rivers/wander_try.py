# Shapes of a wander: python3 wander_try.py OUT  (rows: gradient noise as it is; B-spline value noise with a few settings)
import random, math, sys
from PIL import Image, ImageDraw
def rnd(seed, o, i):
    return random.Random(hash((seed, o, i))).uniform(-1, 1)
def grad(seed, o, x):
    i = math.floor(x); f = x - i
    fade = f*f*f*(f*(f*6-15)+10)
    return 2*(rnd(seed,o,i)*f*(1-fade) + rnd(seed,o,i+1)*(f-1)*fade)
def bsp(seed, o, x):
    i = math.floor(x); t = x - i; s = 1 - t
    w = [s*s*s/6, (4-6*t*t+3*t*t*t)/6, (1+3*t+3*t*t-3*t*t*t)/6, t*t*t/6]
    return sum(w[k]*rnd(seed,o,i+k-1) for k in range(4))
def soft(seed, o, x):
    # A cubic B-spline through gradient-like bumps: control values 0 at the corners, a bump between.
    y = 2*x; i = math.floor(y); t = y - i; q = 1 - t
    w = [q*q*q/6, (4-6*t*t+3*t*t*t)/6, (1+3*t+3*t*t-3*t*t*t)/6, t*t*t/6]
    def c(j):
        if j % 2 == 0: return 0.0
        k = (j - 1)//2
        return 0.75*(rnd(seed,o,k) - rnd(seed,o,k+1))
    return sum(w[k]*c(i+k-1) for k in range(4))
def wander(noise, seed, down, gain, share, octs, fine=2.7):
    s = 0; often = 1; strong = 1
    for o in range(octs):
        s += strong*noise(seed, o, often*down); often *= fine; strong *= share
    x = gain*s
    return x/math.sqrt(1+x*x)
rows = [('grad g2 s.4 o3', grad, 2.0, 0.4, 3), ('grad g1.5 s.3 o3', grad, 1.5, 0.3, 3), ('grad g1.5 s.25 o3', grad, 1.5, 0.25, 3), ('soft g2 s.4 o3', soft, 2.0, 0.4, 3), ('soft g2 s.3 o3', soft, 2.0, 0.3, 3), ('soft g2.5 s.3 o3', soft, 2.5, 0.3, 3), ('soft g2 s.3 o2', soft, 2.0, 0.3, 2)]
W, H, M = 2000, 110, 25.0      # px; 1 px = 2 m; most lean 50 m
im = Image.new('RGB', (W, H*len(rows)), 'white'); d = ImageDraw.Draw(im)
for r, (name, noise, gain, share, octs) in enumerate(rows):
    for seed in (1,):
        pts = [(x, r*H + H/2 + M*wander(noise, seed, x/100.0, gain, share, octs)) for x in range(0, W, 1)]
        d.line(pts, fill=(30,90,160), width=6)
    d.text((5, r*H+5), name, fill='black')
im.save(sys.argv[1])
