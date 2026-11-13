import math, random
SIG=5.67e-8
def column(x,gam,l,hh):
    f=1-math.exp(-x); e=math.exp(-l/hh)
    drop=lambda above: gam*hh*math.log(e+(1-e)*above)
    out=[[0,0],[0,0]]
    if x>0:
        for m in range(2):
            near=-math.log1p(-f*(m+0.5)/2)/x
            out[0][m]=drop(1-near); out[1][m]=drop(near)
    return out,f
def glow(f,tl,lay): return SIG*f/2*sum((tl+d)**4 for d in lay)
AIR=3.57e5
def depth(a,s,c): return a*(math.sqrt(s*c) if s>0 and c>s else c)
gam,l,T=6.5e-3,1e4,288.0
hh=8.314*T/(0.029*9.81)
prata=lambda w: 1-(1+w)*math.exp(-math.sqrt(1.2+3*w))
# shares of the heat light: window 8-12.5 um, the carbon gas's band 12.5-18 um, the vapour's (the rest), at 288 K
def planck_share(lo,hi,T=288):
    h,c,k=6.626e-34,3e8,1.381e-23
    B=lambda l: 1/(l**5*(math.exp(h*c/(l*k*T))-1))
    ls=[(1+0.01*i)*1e-6 for i in range(20000)]
    tot=sum(B(l) for l in ls); return sum(B(l) for l in ls if lo<=l*1e6<hi)/tot
SH=[planck_share(8,12.5), planck_share(12.5,18)]; SH.append(1-SH[0]-SH[1])
print("shares window, carbon band, vapour band", [round(x,3) for x in SH])
def run(p,w,co2=400e-6):
    va,vs,ca,cs,kc=p
    cv=w*555.6; cc=co2*AIR
    taus=[kc*cv*cv/1000.0, depth(ca,cs,cc)+depth(va,vs,cv)*0.3, depth(va,vs,cv)]
    down=0; olr=0; win=0
    for sh,x in zip(SH,taus):
        lay,f=column(x,gam,l,hh)
        down+=sh*glow(f,T,lay[0]); olr+=sh*((1-f)*SIG*T**4+glow(f,T,lay[1])); win+=sh*(1-f)*SIG*T**4
    return down/(SIG*T**4), olr, win
ws=[0.5,0.75,1,1.5,2,2.5,3.5,5]
def err(p):
    if min(p)<=0: return 1e9
    e=sum((run(p,w)[0]-prata(w))**2 for w in ws)/len(ws)
    F=run(p,2.5)[1]-run(p,2.5,800e-6)[1]
    e+=((F-3.7)/3.7*0.05)**2
    e+=((run(p,2.5)[1]-266)/266*0.05)**2
    return e
best=[0.0132,31.6,0.0132,10,0.05]; be=err(best)
random.seed(1)
for it in range(40000):
    q=[x*math.exp(random.gauss(0,0.3 if it<20000 else 0.08)) for x in best]
    eq=err(q)
    if eq<be: best,be=q,eq
p=best
print("fit vapour band %.3e sat %.3g; carbon band %.3e sat %.3g; continuum %.3e (per (mol/m2)^2/1000): rms %.4f"%(p[0],p[1],p[2],p[3],p[4],math.sqrt(be)))
print("eps vs Prata", [(w, round(run(p,w)[0],3), round(prata(w),3)) for w in (0.5,1,2.5,5)])
print("clear OLR w=2.5 %.1f, window to space %.1f, forcing %.2f"%(run(p,2.5)[1], run(p,2.5)[2], run(p,2.5)[1]-run(p,2.5,800e-6)[1]))
