//! Original local-coordinate drafting geometry, using the existing paper strokes.
use std::f64::consts::{PI, TAU};

use crate::{Aabb2, Loop2, Region2, Seg2, Vec2, paper::Item};

use super::{Characteristic, DatumIdentifier, Frame, Invalid, Result, Standard, require};

/// Paper lettering height, independent of view scale. Strokes are h/10; frames are 2h high.
/// Heights outside 1.8–20 mm are rejected as a resource/layout policy, not a standards rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub text_height_mm: f64,
}

impl Default for Style {
    fn default() -> Self {
        Self { text_height_mm: 3.5 }
    }
}

impl Style {
    pub fn validate(self) -> Result<()> {
        require((1.8..=20.0).contains(&self.text_height_mm), "lettering height is finite and 1.8 to 20 mm")
    }
}

/// Complete strokes for one unattached symbol, including its original single-stroke lettering.
/// Local coordinates: x right, y up, frame lower-left at (0,0). Compartment bounds follow
/// the frame's centre lines; painted ink extends h/20 outside them. No font is required.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub size_mm: Vec2,
    pub compartments: Vec<Aabb2>,
    pub items: Vec<Item>,
}

pub(super) fn frame(frame: &Frame, style: Style) -> Result<Layout> {
    let cells = frame.compartments()?;
    style.validate()?;
    let h = style.text_height_mm;
    let mut pen = Pen { h, standard: frame.standard, items: Vec::new() };
    let mut compartments = Vec::with_capacity(cells.len());
    let mut x = 0.0;
    for (i, cell) in cells.iter().enumerate() {
        let width = if i == 0 { 2.0 * h } else { text_width(cell, h, frame.standard)? + 2.0 * inset(h) };
        compartments.push(Aabb2 { min: Vec2::new(x, 0.0), max: Vec2::new(x + width, 2.0 * h) });
        if i == 0 {
            pen.characteristic(frame.characteristic, Vec2::new(h, h));
        } else {
            pen.text(cell, Vec2::new(x + (width - text_width(cell, h, frame.standard)?) / 2.0, h / 2.0))?;
        }
        x += width;
        if i + 1 < cells.len() {
            pen.line(Vec2::new(x, 0.0), Vec2::new(x, 2.0 * h));
        }
    }
    pen.rectangle(x, 2.0 * h);
    Ok(Layout { size_mm: Vec2::new(x, 2.0 * h), compartments, items: pen.items })
}

pub(super) fn datum(datum: &DatumIdentifier, style: Style) -> Result<Layout> {
    datum.validate()?;
    style.validate()?;
    let h = style.text_height_mm;
    // ASME's single-letter square and ISO's rectangular label are both contained here.
    let width = (text_width(&datum.letters, h, datum.standard)? + 2.0 * inset(h)).max(2.0 * h);
    let mut pen = Pen { h, standard: datum.standard, items: Vec::new() };
    pen.rectangle(width, 2.0 * h);
    pen.text(&datum.letters, Vec2::new((width - text_width(&datum.letters, h, datum.standard)?) / 2.0, h / 2.0))?;
    Ok(Layout { size_mm: Vec2::new(width, 2.0 * h), compartments: vec![Aabb2 { min: Vec2::ZERO, max: Vec2::new(width, 2.0 * h) }], items: pen.items })
}

// Clearance is between ink edges. A frame consumes d/2 and a glyph's miter can reach
// 4*(d/2) from its centre line (the paper scene's miter limit), so reserve both first.
fn inset(h: f64) -> f64 {
    let d = h / 10.0;
    (2.0 * d).max(0.7) + 2.5 * d
}

fn glyph_width(c: char, standard: Standard) -> Result<f64> {
    match c {
        'Ⓜ' | 'Ⓛ' | 'Ⓟ' => Ok(1.2),
        '⌀' => Ok(if standard == Standard::Iso1101_2017Iso5459_2024 { 0.7 } else { 0.9 }),
        '.' => Ok(0.2),
        '0'..='9' | 'A'..='Z' => Ok(0.6),
        _ => Err(Invalid("unsupported character in the bounded symbol alphabet")),
    }
}

fn text_width(text: &str, h: f64, standard: Standard) -> Result<f64> {
    let mut width = 0.0;
    for (i, c) in text.chars().enumerate() {
        if i > 0 {
            width += 0.2 * h;
        }
        width += glyph_width(c, standard)? * h;
    }
    Ok(width)
}

struct Pen {
    h: f64,
    standard: Standard,
    items: Vec<Item>,
}

impl Pen {
    fn stroke(&mut self, path: Vec<Seg2>, closed: bool) {
        self.items.push(Item::Stroke { path, closed, width_mm: self.h / 10.0 });
    }

    fn line(&mut self, a: Vec2, b: Vec2) {
        self.stroke(vec![Seg2::Line { a, b }], false);
    }

    fn round_line(&mut self, a: Vec2, b: Vec2) {
        self.line(a, b);
        // Paper strokes have butt caps; analytic fills supply the rounded ends.
        for center in [a, b] {
            self.items.push(Item::Fill { region: Region2 { outer: Loop2::circle(center, self.h / 20.0), holes: Vec::new() } });
        }
    }

    fn path(&mut self, points: &[(f64, f64)], origin: Vec2, scale: f64, closed: bool) {
        let points: Vec<_> = points.iter().map(|&(x, y)| origin + Vec2::new(x, y) * scale).collect();
        let mut path = Vec::with_capacity(points.len());
        for pair in points.windows(2) {
            if let [a, b] = pair {
                path.push(Seg2::Line { a: *a, b: *b });
            }
        }
        if closed && let Some((a, b)) = points.last().zip(points.first()) {
            path.push(Seg2::Line { a: *a, b: *b });
        }
        if !path.is_empty() {
            self.stroke(path, closed);
        }
    }

    fn arc(&mut self, center: Vec2, radius: f64, start: f64, sweep: f64) {
        self.stroke(vec![Seg2::Arc { center, radius, start, sweep }], sweep == TAU);
    }

    fn rectangle(&mut self, width: f64, height: f64) {
        self.path(&[(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)], Vec2::ZERO, 1.0, true);
    }

    fn characteristic(&mut self, characteristic: Characteristic, center: Vec2) {
        use Characteristic::*;
        let h = self.h;
        let at = |x, y| center + Vec2::new(x, y) * h;
        match characteristic {
            Straightness => self.line(at(-0.7, 0.0), at(0.7, 0.0)),
            Flatness => self.path(&[(-0.7, -0.35), (0.3, -0.35), (0.7, 0.35), (-0.3, 0.35)], center, h, true),
            Roundness => self.arc(center, h / 2.0, 0.0, TAU),
            Cylindricity => {
                self.arc(center, 0.4 * h, 0.0, TAU);
                self.line(at(-0.75, -0.5), at(-0.45, 0.5));
                self.line(at(0.45, -0.5), at(0.75, 0.5));
            }
            LineProfile | SurfaceProfile => {
                self.arc(at(0.0, -0.35), 0.7 * h, 0.0, PI);
                if characteristic == SurfaceProfile {
                    self.line(at(-0.7, -0.35), at(0.7, -0.35));
                }
            }
            Parallelism => {
                self.line(at(-0.5, -0.5), at(0.0, 0.5));
                self.line(at(0.0, -0.5), at(0.5, 0.5));
            }
            Perpendicularity => {
                self.line(at(-0.6, -0.5), at(0.6, -0.5));
                self.line(at(0.0, -0.5), at(0.0, 0.5));
            }
            Angularity => self.path(&[(0.65, -0.5), (-0.65, -0.5), (0.05, 0.5)], center, h, false),
            Position => {
                self.arc(center, 0.4 * h, 0.0, TAU);
                self.line(at(-0.65, 0.0), at(0.65, 0.0));
                self.line(at(0.0, -0.65), at(0.0, 0.65));
            }
            Concentricity => {
                self.arc(center, 0.5 * h, 0.0, TAU);
                self.arc(center, 0.25 * h, 0.0, TAU);
            }
            Symmetry => {
                self.line(at(-0.65, 0.0), at(0.65, 0.0));
                for y in [-0.35, 0.35] {
                    self.line(at(-0.45, y), at(0.45, y));
                }
            }
            CircularRunout | TotalRunout => {
                let offsets: &[f64] = if characteristic == TotalRunout { &[-0.2, 0.2] } else { &[0.0] };
                for &x in offsets {
                    self.line(at(x - 0.35, -0.5), at(x + 0.35, 0.5));
                    self.items.push(Item::Fill {
                        region: Region2 { outer: Loop2::polygon(&[at(x - 0.02, 0.35), at(x + 0.35, 0.5), at(x + 0.33, 0.1)]), holes: Vec::new() },
                    });
                }
                if characteristic == TotalRunout {
                    self.line(at(-0.55, -0.5), at(-0.15, -0.5));
                }
            }
        }
    }

    fn text(&mut self, text: &str, mut origin: Vec2) -> Result<()> {
        for c in text.chars() {
            let width = glyph_width(c, self.standard)?;
            match c {
                'Ⓜ' | 'Ⓛ' | 'Ⓟ' => {
                    self.arc(origin + Vec2::new(0.6, 0.5) * self.h, 0.6 * self.h, 0.0, TAU);
                    let letter = match c {
                        'Ⓜ' => 'M',
                        'Ⓛ' => 'L',
                        _ => 'P',
                    };
                    self.letter(letter, origin + Vec2::new(0.405, 0.175) * self.h, 0.65 * self.h)?;
                }
                '⌀' => {
                    if self.standard == Standard::Iso1101_2017Iso5459_2024 {
                        // ISO 7083:2021 Table 6, no. 32: d=h/10, centre-line circle
                        // diameter 6d, diagonal 3d across/9d high, round ends of radius d/2.
                        self.arc(origin + Vec2::new(0.35, 0.5) * self.h, 0.3 * self.h, 0.0, TAU);
                        self.round_line(origin + Vec2::new(0.2, 0.05) * self.h, origin + Vec2::new(0.5, 0.95) * self.h);
                    } else {
                        // Original ASME construction preview; its proportions remain unverified.
                        self.arc(origin + Vec2::new(0.45, 0.5) * self.h, 0.4 * self.h, 0.0, TAU);
                        self.path(&[(0.05, 0.0), (0.85, 1.0)], origin, self.h, false);
                    }
                }
                _ => self.letter(c, origin, self.h)?,
            }
            origin.x += (width + 0.2) * self.h;
        }
        Ok(())
    }

    // Original simple single-stroke construction for the bounded numeric/datum alphabet.
    // This is not extracted from a font, standards figure or another application's artwork.
    fn letter(&mut self, c: char, origin: Vec2, h: f64) -> Result<()> {
        let paths: &[&[(f64, f64)]] = match c {
            '0' => &[&[(0.15, 0.0), (0.0, 0.2), (0.0, 0.8), (0.15, 1.0), (0.45, 1.0), (0.6, 0.8), (0.6, 0.2), (0.45, 0.0), (0.15, 0.0)]],
            '1' => &[&[(0.1, 0.75), (0.3, 1.0), (0.3, 0.0)], &[(0.05, 0.0), (0.55, 0.0)]],
            '2' => &[&[(0.0, 0.8), (0.15, 1.0), (0.45, 1.0), (0.6, 0.8), (0.6, 0.65), (0.0, 0.0), (0.6, 0.0)]],
            '3' => &[&[(0.0, 1.0), (0.6, 1.0), (0.3, 0.55), (0.6, 0.35), (0.6, 0.15), (0.45, 0.0), (0.0, 0.0)]],
            '4' => &[&[(0.45, 0.0), (0.45, 1.0), (0.0, 0.3), (0.6, 0.3)]],
            '5' => &[&[(0.6, 1.0), (0.0, 1.0), (0.0, 0.55), (0.45, 0.55), (0.6, 0.4), (0.6, 0.15), (0.45, 0.0), (0.0, 0.0)]],
            '6' => {
                &[&[(0.6, 1.0), (0.15, 1.0), (0.0, 0.75), (0.0, 0.15), (0.15, 0.0), (0.45, 0.0), (0.6, 0.15), (0.6, 0.4), (0.45, 0.55), (0.0, 0.55)]]
            }
            '7' => &[&[(0.0, 1.0), (0.6, 1.0), (0.15, 0.0)]],
            '8' => &[&[
                (0.15, 0.5),
                (0.0, 0.7),
                (0.0, 0.85),
                (0.15, 1.0),
                (0.45, 1.0),
                (0.6, 0.85),
                (0.6, 0.7),
                (0.45, 0.5),
                (0.15, 0.5),
                (0.0, 0.3),
                (0.0, 0.15),
                (0.15, 0.0),
                (0.45, 0.0),
                (0.6, 0.15),
                (0.6, 0.3),
                (0.45, 0.5),
            ]],
            '9' => {
                &[&[(0.6, 0.45), (0.15, 0.45), (0.0, 0.6), (0.0, 0.85), (0.15, 1.0), (0.45, 1.0), (0.6, 0.85), (0.6, 0.25), (0.45, 0.0), (0.0, 0.0)]]
            }
            '.' => {
                self.arc(origin + Vec2::new(0.1, 0.05) * h, 0.05 * h, 0.0, TAU);
                return Ok(());
            }
            'A' => &[&[(0.0, 0.0), (0.3, 1.0), (0.6, 0.0)], &[(0.15, 0.5), (0.45, 0.5)]],
            'B' => &[&[(0.0, 0.0), (0.0, 1.0), (0.4, 1.0), (0.6, 0.8), (0.4, 0.5), (0.0, 0.5)], &[(0.4, 0.5), (0.6, 0.3), (0.4, 0.0), (0.0, 0.0)]],
            'C' => &[&[(0.6, 0.85), (0.45, 1.0), (0.15, 1.0), (0.0, 0.8), (0.0, 0.2), (0.15, 0.0), (0.45, 0.0), (0.6, 0.15)]],
            'D' => &[&[(0.0, 0.0), (0.0, 1.0), (0.3, 1.0), (0.6, 0.7), (0.6, 0.3), (0.3, 0.0), (0.0, 0.0)]],
            'E' => &[&[(0.6, 1.0), (0.0, 1.0), (0.0, 0.0), (0.6, 0.0)], &[(0.0, 0.5), (0.45, 0.5)]],
            'F' => &[&[(0.0, 0.0), (0.0, 1.0), (0.6, 1.0)], &[(0.0, 0.5), (0.45, 0.5)]],
            'G' => &[&[(0.6, 0.85), (0.45, 1.0), (0.15, 1.0), (0.0, 0.8), (0.0, 0.2), (0.15, 0.0), (0.6, 0.0), (0.6, 0.5), (0.35, 0.5)]],
            'H' => &[&[(0.0, 0.0), (0.0, 1.0)], &[(0.6, 0.0), (0.6, 1.0)], &[(0.0, 0.5), (0.6, 0.5)]],
            'J' => &[&[(0.0, 0.2), (0.15, 0.0), (0.4, 0.0), (0.55, 0.2), (0.55, 1.0)]],
            'K' => &[&[(0.0, 0.0), (0.0, 1.0)], &[(0.6, 1.0), (0.0, 0.45), (0.6, 0.0)]],
            'L' => &[&[(0.0, 1.0), (0.0, 0.0), (0.6, 0.0)]],
            'M' => &[&[(0.0, 0.0), (0.0, 1.0), (0.3, 0.45), (0.6, 1.0), (0.6, 0.0)]],
            'N' => &[&[(0.0, 0.0), (0.0, 1.0), (0.6, 0.0), (0.6, 1.0)]],
            'P' => &[&[(0.0, 0.0), (0.0, 1.0), (0.4, 1.0), (0.6, 0.8), (0.6, 0.65), (0.4, 0.5), (0.0, 0.5)]],
            'R' => &[&[(0.0, 0.0), (0.0, 1.0), (0.4, 1.0), (0.6, 0.8), (0.6, 0.65), (0.4, 0.5), (0.0, 0.5)], &[(0.3, 0.5), (0.6, 0.0)]],
            'S' => {
                &[&[(0.6, 0.85), (0.45, 1.0), (0.15, 1.0), (0.0, 0.8), (0.0, 0.65), (0.6, 0.35), (0.6, 0.2), (0.45, 0.0), (0.15, 0.0), (0.0, 0.15)]]
            }
            'T' => &[&[(0.0, 1.0), (0.6, 1.0)], &[(0.3, 1.0), (0.3, 0.0)]],
            'U' => &[&[(0.0, 1.0), (0.0, 0.2), (0.15, 0.0), (0.45, 0.0), (0.6, 0.2), (0.6, 1.0)]],
            'V' => &[&[(0.0, 1.0), (0.3, 0.0), (0.6, 1.0)]],
            'W' => &[&[(0.0, 1.0), (0.15, 0.0), (0.3, 0.55), (0.45, 0.0), (0.6, 1.0)]],
            'X' => &[&[(0.0, 0.0), (0.6, 1.0)], &[(0.0, 1.0), (0.6, 0.0)]],
            'Y' => &[&[(0.0, 1.0), (0.3, 0.5), (0.6, 1.0)], &[(0.3, 0.5), (0.3, 0.0)]],
            'Z' => &[&[(0.0, 1.0), (0.6, 1.0), (0.0, 0.0), (0.6, 0.0)]],
            _ => return Err(Invalid("unsupported character in the bounded symbol alphabet")),
        };
        for points in paths {
            self.path(points, origin, h, false);
        }
        Ok(())
    }
}
