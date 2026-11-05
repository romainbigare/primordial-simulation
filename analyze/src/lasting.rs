//! Patterns that last over the frames: groups (dense animals) and lanes (worn ground). Both are one rule: a square is
//! on in a frame, it stays on for a run of frames (`LAST_FRAMES` frames and `LAST_YEARS` years), and the squares with
//! such a run that touch (eight neighbours) form one pattern.

use crate::frames::Frames;
use crate::{GROUP_DENSITY, GROUP_MIN_SQUARES, LANE_EASE, LANE_MIN_SQUARES, LAST_FRAMES, LAST_YEARS};
use serde_json::{json, Value};
use std::io;

/// The longest run of consecutive frames a square is on, first and last frame.
type Run = (usize, usize);

/// For each square its longest run of frames in `on` (frame-major, one entry a square and frame) that lasts.
fn runs(f: &Frames, on: &[bool]) -> Vec<Option<Run>> {
    let (n, frames) = (f.squares(), f.years.len());
    (0..n)
        .map(|s| {
            let mut best: Option<Run> = None;
            let mut start = None;
            for t in 0..=frames {
                match (t < frames && on[t * n + s], start) {
                    (true, None) => start = Some(t),
                    (false, Some(a)) => {
                        let last = t - 1;
                        let lasts = t - a >= LAST_FRAMES && f.years[last] - f.years[a] >= LAST_YEARS;
                        if lasts && best.is_none_or(|(b, e)| e - b < last - a) {
                            best = Some((a, last));
                        }
                        start = None;
                    }
                    _ => {}
                }
            }
            best
        })
        .collect()
}

/// The squares that have a run, grouped by touching (a column wraps east to west on a globe); each group lists its squares
/// in order, and the groups are in order of their first square.
fn components(f: &Frames, runs: &[Option<Run>], least: usize) -> Vec<Vec<usize>> {
    let (w, h) = (f.w as isize, f.h as isize);
    let mut seen = vec![false; runs.len()];
    let mut out = vec![];
    for first in (0..runs.len()).filter(|&s| runs[s].is_some()) {
        if std::mem::replace(&mut seen[first], true) {
            continue;
        }
        let (mut stack, mut comp) = (vec![first], vec![]);
        while let Some(s) = stack.pop() {
            comp.push(s);
            let (x, y) = ((s % f.w) as isize, (s / f.w) as isize);
            for (dx, dy) in (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (dx, dy))) {
                let (nx, ny) = (if f.wrap { (x + dx).rem_euclid(w) } else { x + dx }, y + dy);
                if (0..w).contains(&nx) && (0..h).contains(&ny) {
                    let q = (ny * w + nx) as usize;
                    if runs[q].is_some() && !std::mem::replace(&mut seen[q], true) {
                        stack.push(q);
                    }
                }
            }
        }
        comp.sort_unstable();
        if comp.len() >= least {
            out.push(comp);
        }
    }
    out
}

/// What every card of a pattern that lasts holds: the squares, the years of its longest run, and when it began.
fn card(f: &Frames, id: String, category: &str, comp: &[usize], runs: &[Option<Run>]) -> Value {
    let all: Vec<Run> = comp.iter().filter_map(|&s| runs[s]).collect();
    let longest = all.iter().max_by(|a, b| (a.1 - a.0).cmp(&(b.1 - b.0)).then(b.0.cmp(&a.0))).expect("a pattern has a run");
    let first = all.iter().map(|r| r.0).min().expect("a pattern has a run");
    json!({
        "pattern_id": id,
        "category": category,
        "cells": comp.len(),
        "years": f.years[longest.1] - f.years[longest.0],
        "first_seen_year": f.years[first],
        "evidence_refs": [format!("frame:{}", f.years[first]), format!("frame:{}", f.years[longest.1])],
    })
}

/// Lasting clusters: squares where the animals' mass per m² is over `GROUP_DENSITY` times the mean of the frame, for a run
/// of frames; a group of at least `GROUP_MIN_SQUARES` squares. Adds the centre (square units) and the bodies at the peak.
pub fn groups(f: &Frames) -> io::Result<Vec<Value>> {
    let n = f.squares();
    let mut on = Vec::with_capacity(n * f.years.len());
    for t in 0..f.years.len() {
        let density = f.layer(t, "animals")?;
        let mean = density.iter().sum::<f64>() / n as f64;
        on.extend(density.iter().map(|&d| mean > 0.0 && d > GROUP_DENSITY * mean));
    }
    let runs = runs(f, &on);
    let comps = components(f, &runs, GROUP_MIN_SQUARES);
    let mut peak = vec![0.0f64; comps.len()];
    for t in 0..f.years.len() {
        let bodies = f.layer(t, "bodies")?;
        for (p, comp) in peak.iter_mut().zip(&comps) {
            *p = p.max(comp.iter().filter(|&&s| runs[s].is_some_and(|r| (r.0..=r.1).contains(&t))).map(|&s| bodies[s]).sum());
        }
    }
    Ok(comps
        .iter()
        .zip(peak)
        .map(|(comp, peak)| {
            let centre = [0, 1].map(|axis| comp.iter().map(|&s| if axis == 0 { s % f.w } else { s / f.w } as f64).sum::<f64>() / comp.len() as f64);
            let mut c = card(f, format!("group-{}", comp[0]), "group", comp, &runs);
            c["peak_bodies"] = json!(peak.round());
            c["centre"] = json!(centre);
            c
        })
        .collect())
}

/// Lasting lanes: squares whose worn layer is at least `LANE_EASE` for a run of frames; a path of at least
/// `LANE_MIN_SQUARES` squares. Adds its two ends (the farthest apart of its squares) and the mean traffic on it.
pub fn lanes(f: &Frames) -> io::Result<Vec<Value>> {
    let n = f.squares();
    let mut on = Vec::with_capacity(n * f.years.len());
    for t in 0..f.years.len() {
        on.extend(f.layer(t, "worn")?.iter().map(|&e| e >= LANE_EASE));
    }
    let runs = runs(f, &on);
    let comps = components(f, &runs, LANE_MIN_SQUARES);
    let mut traffic = vec![(0.0, 0usize); comps.len()];
    for t in 0..f.years.len() {
        let layer = f.layer(t, "traffic")?;
        for (sum, comp) in traffic.iter_mut().zip(&comps) {
            for &s in comp.iter().filter(|&&s| runs[s].is_some_and(|r| (r.0..=r.1).contains(&t))) {
                *sum = (sum.0 + layer[s], sum.1 + 1);
            }
        }
    }
    Ok(comps
        .iter()
        .zip(traffic)
        .map(|(comp, (sum, count))| {
            let far = |from: usize| *comp.iter().max_by_key(|&&s| (apart(f, from, s), std::cmp::Reverse(s))).expect("a lane has squares");
            let (a, b) = (far(comp[0]), far(far(comp[0])));
            let at = |s: usize| [s % f.w, s / f.w];
            let (a, b) = (a.min(b), a.max(b));
            let mut c = card(f, format!("route-{}", comp[0]), "route", comp, &runs);
            c["ends"] = json!([at(a), at(b)]);
            c["mean_traffic"] = json!(sum / count as f64);
            c
        })
        .collect())
}

/// The squared distance between squares `a` and `b`, in squares (east-west round a globe).
fn apart(f: &Frames, a: usize, b: usize) -> usize {
    let dx = (a % f.w).abs_diff(b % f.w);
    let dx = if f.wrap { dx.min(f.w - dx) } else { dx };
    dx * dx + (a / f.w).abs_diff(b / f.w).pow(2)
}
