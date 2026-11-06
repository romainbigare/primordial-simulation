//! The camera of a run (laws.md 0): from given months on, a window of the map and the depth of the cells it asks for
//! there. It is an input of the run, as the placements of life are: it goes with the run's plan into a snapshot and
//! into the tape, so the same seed and the same camera path give the same world. Refinement reads it
//! (`World::refine`); no law does.

use crate::space::tree::DEPTH_MOST;
use serde::{Deserialize, Serialize};

/// From month `month` on (whole months since the start): the window of the map from column `x` and row `y` (its
/// north-west corner), `w` squares wide and `h` high, asks for cells of `depth` below their air block (a square is
/// `Square::square_depth`); a window of no area asks for nothing, and with `rest` also brings the budget of cells down
/// to the cells as built (`Camera::rests`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    pub month: u64,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub depth: u8,
    pub rest: bool,
}

/// The shots of a run in the order of their months, each in force until the next (none: a run with no camera).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub shots: Vec<Shot>,
}

impl Camera {
    /// The shot in force at month `month`: the last that starts by then, if its window has an area.
    pub fn at(&self, month: u64) -> Option<&Shot> {
        self.in_force(month).filter(|s| s.w > 0.0 && s.h > 0.0)
    }
    /// Whether the shot in force at month `month` rests: the budget of cells is then the cells as built, so the world
    /// steps as the coarse mesh does while nobody looks.
    pub fn rests(&self, month: u64) -> bool {
        self.in_force(month).is_some_and(|s| s.rest)
    }
    fn in_force(&self, month: u64) -> Option<&Shot> {
        self.shots.iter().take_while(|s| s.month <= month).last()
    }
}

impl Shot {
    /// Whether its window overlaps the box from (`x0`, `y0`) to (`x1`, `y1`), in squares; round a globe of `wrap`
    /// columns (0: a flat map) east and west.
    pub fn sees(&self, [x0, y0, x1, y1]: [f64; 4], wrap: f64) -> bool {
        let rows = y0 < self.y + self.h && self.y < y1;
        let turns: &[f64] = if wrap > 0.0 { &[-1.0, 0.0, 1.0] } else { &[0.0] };
        rows && turns.iter().any(|t| x0 < self.x + t * wrap + self.w && self.x + t * wrap < x1)
    }
}

/// A camera path as text: one shot a line, `MONTH X Y W H DEPTH` (`Shot`), or `MONTH` alone, which sends the camera
/// away from that month, or `MONTH rest`, which also brings the budget back to the cells as built; the months rise from
/// line to line. Blank lines and what follows a `#` are skipped.
impl std::str::FromStr for Camera {
    type Err = String;
    fn from_str(text: &str) -> Result<Camera, String> {
        let mut shots: Vec<Shot> = Vec::new();
        for (k, line) in text.lines().enumerate() {
            let words: Vec<&str> = line.split('#').next().unwrap_or("").split_whitespace().collect();
            if words.is_empty() {
                continue;
            }
            let bad = |what: &str| format!("camera line {}: {what}: {line:?}", k + 1);
            let num = |i: usize| words[i].parse::<f64>().map_err(|_| bad("not a number"));
            let month = words[0].parse::<u64>().map_err(|_| bad("the month is not a whole number"))?;
            let shot = match words.len() {
                1 => Shot { month, x: 0.0, y: 0.0, w: 0.0, h: 0.0, depth: 0, rest: false },
                2 if words[1] == "rest" => Shot { month, x: 0.0, y: 0.0, w: 0.0, h: 0.0, depth: 0, rest: true },
                6 => {
                    let depth = words[5].parse::<u8>().ok().filter(|&d| d <= DEPTH_MOST).ok_or_else(|| bad(&format!("the depth is not 0 to {DEPTH_MOST}")))?;
                    Shot { month, x: num(1)?, y: num(2)?, w: num(3)?, h: num(4)?, depth, rest: false }
                }
                _ => return Err(bad("a line is MONTH X Y W H DEPTH, MONTH or MONTH rest")),
            };
            if [shot.x, shot.y, shot.w, shot.h].iter().any(|v| !v.is_finite() || *v < 0.0) {
                return Err(bad("the window is not at 0 or more"));
            }
            if shots.last().is_some_and(|s| s.month >= month) {
                return Err(bad("the months do not rise"));
            }
            shots.push(shot);
        }
        Ok(Camera { shots })
    }
}
