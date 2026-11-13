"""The cloud's two-stream (delta-Eddington, Meador and Weaver 1980; Joseph et al. 1976) against Slingo's (1989) four bands
for drops of 10 um: what one grey near-infrared co-albedo gives."""
import math

def ts(tau, w0, g, mu0):
    # delta scaling
    f = g * g
    tau = (1 - w0 * f) * tau
    w0 = (1 - f) * w0 / (1 - w0 * f)
    g = g / (1 + g)
    g3 = (2 - 3 * g * mu0) / 4
    g4 = 1 - g3
    direct = math.exp(-tau / mu0)
    if 1 - w0 < 1e-9:
        g1 = 3 * (1 - g) / 4
        R = (g1 * tau + (g3 - g1 * mu0) * (1 - direct)) / (1 + g1 * tau)
        return R, 1 - R, direct
    g1 = (7 - w0 * (4 + 3 * g)) / 4
    g2 = -(1 - w0 * (4 - 3 * g)) / 4
    k = math.sqrt(g1 * g1 - g2 * g2)
    a1 = g1 * g4 + g2 * g3
    a2 = g1 * g3 + g2 * g4
    if abs(1 - k * mu0) < 1e-6:
        mu0 += 1e-5
    e = math.exp(-2 * k * tau)  # scaled by exp(-k tau) top and bottom
    den = (1 - k * k * mu0 * mu0) * ((k + g1) + (k - g1) * e)
    R = w0 / den * ((1 - k * mu0) * (a2 + k * g3) - (1 + k * mu0) * (a2 - k * g3) * e - 2 * k * (g3 - a2 * mu0) * direct * math.exp(-k * tau))
    T = direct * (1 - w0 / den * ((1 + k * mu0) * (a1 + k * g4) - (1 - k * mu0) * (a1 - k * g4) * e - 2 * k * (g4 + a1 * mu0) * math.exp(-k * tau) / direct))
    return R, T, direct

RE = 10.0
BANDS = [  # share of the sun, tau per g/m2, 1 - w0, g  (Slingo 1989, as coded in GFDL's cloud_rad.F90)
    (0.459760, 0.02817 + 1.305 / RE, -5.62e-8 + 1.63e-7 * RE, 0.829 + 2.482e-3 * RE),
    (0.326158, 0.02682 + 1.346 / RE, -6.94e-6 + 2.35e-5 * RE, 0.794 + 4.226e-3 * RE),
    (0.180608, 0.02264 + 1.454 / RE, 4.64e-4 + 1.24e-3 * RE, 0.754 + 6.560e-3 * RE),
    (0.033474, 0.01281 + 1.641 / RE, 2.01e-1 + 7.56e-3 * RE, 0.826 + 4.353e-3 * RE),
]

def four(W, mu0):
    R = T = 0
    for s, kt, co, g in BANDS:
        r, t, _ = ts(W * kt, 1 - co, g, mu0)
        R += s * r; T += s * t
    return R, T

def grey(W, mu0, co, g=0.85, r=RE):
    tau = 3 * W * 1e-3 / (2 * 1000 * r * 1e-6)
    r_, t_, _ = ts(tau, 1 - co, g, mu0)
    return r_, t_

def two(W, mu0, co, share, g=0.85, r=RE):
    tau = 3 * W * 1e-3 / (2 * 1000 * r * 1e-6)
    r1, t1, _ = ts(tau, 1.0, g, mu0)
    r2, t2, _ = ts(tau, 1 - co, g, mu0)
    return (1 - share) * r1 + share * r2, (1 - share) * t1 + share * t2

if __name__ == '__main__':
    cases = [(W, mu) for W in (10, 30, 100, 300, 1000) for mu in (0.3, 0.6, 1.0)]
    print("W g/m2  mu0   four bands R  A |")
    for W, mu in cases:
        R, T = four(W, mu)
        print(f"{W:6} {mu:4}  {R:.3f} {1-R-T:.3f}")
    # the fits
    def err(model):
        return math.sqrt(sum((math.log(max(1e-9, 1 - sum(model(W, mu)))) - math.log(1 - sum(four(W, mu)))) ** 2 for W, mu in cases) / len(cases))
    best = min(((err(lambda W, mu: grey(W, mu, co)), co) for co in [10 ** (x / 50) for x in range(-200, -50)]))
    print("one grey co-albedo:", best)
    for share in (0.54024, 0.214082):
        b2 = min(((err(lambda W, mu: two(W, mu, co, share)), co) for co in [10 ** (x / 50) for x in range(-200, 0)]))
        print("two parts, share", share, b2)
    for W, mu in cases:
        R, T = four(W, mu)
        g1 = grey(W, mu, best[1]); t2 = two(W, mu, b2[1], 0.214082)
        print(f"{W:6} {mu:4}  four R {R:.3f} A {1-R-T:.4f} | grey R {g1[0]:.3f} A {1-sum(g1):.4f} | two R {t2[0]:.3f} A {1-sum(t2):.4f}")
