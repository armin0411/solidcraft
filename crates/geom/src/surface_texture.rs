//! Surface texture indications, independent of model surfaces and sheet placement.
//!
//! ISO 21920-1:2021 and ASME Y14.36-2018 are distinct specifications; changing a drawing's
//! profile must not reinterpret these records. The supported ISO subset is the eight parameters
//! with defined defaults. No ISO 1302 import or implicit acceptance-rule conversion is provided.
//! [`Indication::text`] is an explicit, lossless human-readable description, not ASME's normative
//! textual presentation grammar. [`Indication::preview_geometry`] produces original construction strokes
//! and text slots in local paper millimetres. See `docs/surface-texture.md` for evidence limits:
//! these constructions are previews, not a certificate of standard-conforming proportions.
//!
//! Public records can be edited or deserialized without validation. Text and geometry methods
//! revalidate. Derived serialization does not validate; validate before encoding too. Transport
//! callers must also cap encoded bytes/nesting before serde allocates.

use std::f64::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

use crate::{Seg2, Vec2, paper::Item};

mod proportions;

/// Resource limits, not limits imposed by a drafting standard.
pub const MAX_PARAMETER_LINES: usize = 16;
pub const MAX_PROCESS_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    AnyProcess,
    RemovalRequired,
    RemovalProhibited,
}

/// Lay relative to the plane of projection. Feature-referenced directions are later work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lay {
    Parallel,
    Perpendicular,
    Crossed,
    Multidirectional,
    Circular,
    Radial,
    Particulate,
}

impl Lay {
    pub fn text(self) -> &'static str {
        match self {
            Self::Parallel => "parallel",
            Self::Perpendicular => "perpendicular",
            Self::Crossed => "crossed",
            Self::Multidirectional => "multidirectional",
            Self::Circular => "circular",
            Self::Radial => "radial",
            Self::Particulate => "particulate",
        }
    }
}

/// ISO parameters for which ISO 21920-3 supplies default settings (study §11.31).
/// Other parameters require operators not implemented by this subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsoParameter {
    Ra,
    Rz,
    Rp,
    Rv,
    Rq,
    Rmax,
    Rt,
    Pt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToleranceType {
    Upper,
    Lower,
}

/// Always explicit in data: absence of a printed rule must not become a legacy 16% default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceRule {
    Maximum,
    SixteenPercent,
    Median,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsoParameterLine {
    pub parameter: IsoParameter,
    pub tolerance: ToleranceType,
    /// Parameter amplitude in micrometres. Not a length in model millimetres.
    pub limit_um: f64,
    pub acceptance: AcceptanceRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmplitudeUnit {
    Micrometre,
    Microinch,
}

/// A deliberately small ASME amplitude subset. Wavelength domains and filters require a later
/// extension, rather than accepting uninterpreted text as a validated parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsmeParameter {
    Ra,
    Rq,
    Rz,
    Rt,
    Rp,
    Rv,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsmeParameterLine {
    pub parameter: AsmeParameter,
    /// A maximum when `minimum` is absent, otherwise the upper end of an inclusive range.
    pub maximum: f64,
    pub minimum: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "standard", rename_all = "snake_case", deny_unknown_fields)]
pub enum Specification {
    #[serde(rename = "iso_21920_1_2021")]
    Iso21920_1_2021 { lines: Vec<IsoParameterLine> },
    #[serde(rename = "asme_y14_36_2018")]
    AsmeY14_36_2018 { unit: AmplitudeUnit, lines: Vec<AsmeParameterLine> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Indication {
    pub specification: Specification,
    pub kind: SymbolKind,
    pub lay: Option<Lay>,
    /// A single plain-text line, never markup, a font, or a grammar escape hatch.
    pub process: Option<String>,
    /// Retained as semantic intent. This module does not attach the mark to a leader.
    pub all_round: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("surface texture: {0}")]
pub struct Invalid(pub &'static str);

type Result<T> = std::result::Result<T, Invalid>;

fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(Invalid(reason)) }
}

/// An arithmetic/resource bound, not a standard's maximum roughness value.
fn amplitude(n: f64) -> bool {
    n.is_finite() && n > 0.0 && n <= 1e6
}

impl Indication {
    pub fn validate(&self) -> Result<()> {
        let count = match &self.specification {
            Specification::Iso21920_1_2021 { lines } => lines.len(),
            Specification::AsmeY14_36_2018 { lines, .. } => lines.len(),
        };
        require((1..=MAX_PARAMETER_LINES).contains(&count), "one to sixteen parameter lines are required")?;
        if let Some(process) = &self.process {
            require(!process.is_empty() && process.len() <= MAX_PROCESS_BYTES, "process must contain one to 256 UTF-8 bytes")?;
            // Reject controls, format controls/bidi overrides and non-ASCII whitespace; never trim
            // or normalize the user's process into a different instruction.
            require(
                process.trim() == process && process.chars().all(plain_char),
                "process must be plain single-line text without controls or surrounding whitespace",
            )?;
        }
        match &self.specification {
            Specification::Iso21920_1_2021 { lines } => {
                for (i, line) in lines.iter().enumerate() {
                    require(amplitude(line.limit_um), "ISO limit must be finite, above zero and at most 1e6 micrometres")?;
                    for previous in lines.iter().take(i).filter(|p| p.parameter == line.parameter) {
                        require(previous.tolerance != line.tolerance, "duplicate ISO parameter and tolerance type")?;
                        require(previous.acceptance == line.acceptance, "paired ISO limits must use the same acceptance rule")?;
                        let (lower, upper) = if line.tolerance == ToleranceType::Lower {
                            (line.limit_um, previous.limit_um)
                        } else {
                            (previous.limit_um, line.limit_um)
                        };
                        require(lower <= upper, "ISO lower limit exceeds upper limit")?;
                    }
                }
            }
            Specification::AsmeY14_36_2018 { lines, .. } => {
                for (i, line) in lines.iter().enumerate() {
                    require(amplitude(line.maximum), "ASME maximum must be finite, above zero and at most 1e6")?;
                    require(
                        line.minimum.is_none_or(|n| amplitude(n) && n <= line.maximum),
                        "ASME minimum must be positive, finite and no greater than maximum",
                    )?;
                    require(!lines.iter().take(i).any(|p| p.parameter == line.parameter), "duplicate ASME parameter")?;
                }
            }
        }
        Ok(())
    }

    /// Ordered, standard-specific parameter lines; ISO ordering controls default settings.
    /// These are display fragments; the complete diagnostic description is [`Self::text`].
    pub fn parameter_texts(&self) -> Result<Vec<String>> {
        self.validate()?;
        Ok(match &self.specification {
            Specification::Iso21920_1_2021 { lines } => lines
                .iter()
                .map(|line| {
                    let tolerance = if line.tolerance == ToleranceType::Lower { "L " } else { "" };
                    let rule = match line.acceptance {
                        AcceptanceRule::Maximum => "",
                        AcceptanceRule::SixteenPercent => " T16%",
                        AcceptanceRule::Median => " Tmed",
                    };
                    format!("{tolerance}{:?} {}{rule}", line.parameter, line.limit_um)
                })
                .collect(),
            Specification::AsmeY14_36_2018 { lines, .. } => lines
                .iter()
                .map(|line| match line.minimum {
                    Some(minimum) => format!("{:?} {minimum}-{}", line.parameter, line.maximum),
                    None => format!("{:?} {}", line.parameter, line.maximum),
                })
                .collect(),
        })
    }

    /// Explicit description with standard, units, removal, ordered parameters, lay and process.
    /// Delimiters in process text are quoted using Rust string escaping.
    pub fn text(&self) -> Result<String> {
        let lines = self.parameter_texts()?;
        let standard = match self.specification {
            Specification::Iso21920_1_2021 { .. } => "ISO 21920-1:2021; um; default rule=maximum",
            Specification::AsmeY14_36_2018 { unit: AmplitudeUnit::Micrometre, .. } => "ASME Y14.36-2018; um",
            Specification::AsmeY14_36_2018 { unit: AmplitudeUnit::Microinch, .. } => "ASME Y14.36-2018; uin",
        };
        let kind = match self.kind {
            SymbolKind::AnyProcess => "any process",
            SymbolKind::RemovalRequired => "material removal required",
            SymbolKind::RemovalProhibited => "material removal prohibited",
        };
        let mut text = format!("{standard}; {kind}; {}", lines.join(" / "));
        if let Some(lay) = self.lay {
            text.push_str(&format!("; lay={}", lay.text()));
        }
        if let Some(process) = &self.process {
            text.push_str(&format!("; process={process:?}"));
        }
        if self.all_round {
            text.push_str("; all-round");
        }
        Ok(text)
    }
}

fn plain_char(c: char) -> bool {
    !c.is_control()
        && (!c.is_whitespace() || c == ' ')
        && !matches!(c, '\u{00ad}' | '\u{0600}'..='\u{0605}' | '\u{061c}' | '\u{06dd}' | '\u{070f}' | '\u{0890}'..='\u{0891}' | '\u{08e2}' | '\u{180e}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}' | '\u{fff9}'..='\u{fffb}' | '\u{110bd}' | '\u{110cd}' | '\u{13430}'..='\u{1343f}' | '\u{1bca0}'..='\u{1bca3}' | '\u{1d173}'..='\u{1d17a}' | '\u{e0000}'..='\u{e007f}')
}

/// Text is returned separately because `paper::Item` has no text primitive or font metrics yet.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    /// Left baseline in local coordinates, x right, y up.
    pub baseline_mm: Vec2,
    pub height_mm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolGeometry {
    pub strokes: Vec<Item>,
    pub parameters: Vec<TextRun>,
    pub process: Option<TextRun>,
    /// All-round preview circle in the same coordinates as `strokes`, at the reference-line
    /// corner. Its diameter and the ISO convention still require standards review.
    pub all_round_mark: Vec<Item>,
}

impl Indication {
    /// Original construction preview with its apex at the origin. No view scale, rotation,
    /// leader or model attachment is accepted here. `reference_width_mm` comes from a future
    /// text measurer; it must span all text (this module cannot verify font widths).
    ///
    /// ISO accepts only nominal text heights 2.5, 3.5, 5, 7, 10, 14 and 20 mm; ASME accepts
    /// finite heights in 2.5..=20 mm. Both retain unchecked preview proportions, documented
    /// in `docs/surface-texture.md`. ISO's distinguishing bar is not the removal crossbar.
    pub fn preview_geometry(&self, height_mm: f64, reference_width_mm: f64) -> Result<SymbolGeometry> {
        let texts = self.parameter_texts()?;
        let iso = matches!(self.specification, Specification::Iso21920_1_2021 { .. });
        let dimensions = proportions::resolve(iso, height_mm, texts.len())?;
        require(
            reference_width_mm.is_finite() && (height_mm..=1000.0).contains(&reference_width_mm),
            "reference width must be finite, at least the text height and at most 1000 mm",
        )?;
        let h = height_mm;
        let profile = dimensions.profile;
        let short = dimensions.short;
        let top = dimensions.top;
        let slope = proportions::LEG_ANGLE_RAD.tan();
        let right = top / slope;
        let corner = Vec2::new(right, top);
        let width = dimensions.stroke;
        let margin = profile.text_margin_h * h;
        let lay_width = if self.lay == Some(Lay::Circular) { profile.circular_lay_width_h } else { profile.lay_width_h } * h;
        let lay_origin = Vec2::new(right + reference_width_mm + profile.lay_gap_h * h, top + profile.lay_baseline_h * h);
        let mut end = right + reference_width_mm + 2.0 * margin;
        if self.lay.is_some() {
            end = end.max(lay_origin.x + lay_width + margin);
        }
        // One connected stroke gives the tip and shoulder the paper scene's miter joins.
        // Three separately butt-capped lines would leave notches at these corners.
        let mut strokes = vec![Item::Stroke {
            path: vec![
                Seg2::Line { a: Vec2::new(-short / slope, short), b: Vec2::ZERO },
                Seg2::Line { a: Vec2::ZERO, b: corner },
                Seg2::Line { a: corner, b: Vec2::new(end, top) },
            ],
            closed: false,
            width_mm: width,
        }];
        let mut line = |a, b| strokes.push(stroke(Seg2::Line { a, b }, width, false));
        if self.kind == SymbolKind::RemovalRequired {
            line(Vec2::new(-short / slope, short), Vec2::new(short / slope, short));
        }
        if iso {
            let half = dimensions.identifier_length / 2.0;
            line(Vec2::new(-half, dimensions.identifier_height), Vec2::new(half, dimensions.identifier_height));
        }
        if self.kind == SymbolKind::RemovalProhibited {
            // Circle tangent to the two 60-degree legs, strictly below the crossbar location.
            let radius = profile.prohibited_radius_h * h;
            strokes.push(circle(Vec2::new(0.0, 2.0 * radius), radius, width));
        }
        if let Some(lay) = self.lay {
            strokes.extend(lay_strokes(lay, lay_origin, lay_width, profile.lay_height_h * h, width));
        }
        let parameters = texts
            .into_iter()
            .enumerate()
            .map(|(i, text)| TextRun {
                text,
                baseline_mm: Vec2::new(right + margin, top - (profile.parameter_top_gap_h + i as f64 * profile.line_pitch_h) * h),
                height_mm: h,
            })
            .collect();
        let process = self.process.as_ref().map(|text| TextRun {
            text: text.clone(),
            baseline_mm: Vec2::new(right + margin, top + profile.process_baseline_h * h),
            height_mm: h,
        });
        // ASME supports the corner anchor. For ISO this remains a provisional interpretation
        // of the stored intent, not a verified ISO 21920 modifier. Both diameters are unchecked.
        let all_round_mark = if self.all_round { vec![circle(corner, profile.all_round_radius_h * h, width)] } else { Vec::new() };
        Ok(SymbolGeometry { strokes, parameters, process, all_round_mark })
    }
}

fn stroke(seg: Seg2, width_mm: f64, closed: bool) -> Item {
    Item::Stroke { path: vec![seg], width_mm, closed }
}

fn circle(center: Vec2, radius: f64, width: f64) -> Item {
    stroke(Seg2::Arc { center, radius, start: 0.0, sweep: TAU }, width, true)
}

/// Original line/arc constructions, including letter-shaped marks; no font glyphs or assets.
fn lay_strokes(lay: Lay, origin: Vec2, w: f64, h: f64, width: f64) -> Vec<Item> {
    let mut strokes = Vec::new();
    let mut line = |x0, y0, x1, y1| {
        strokes.push(stroke(Seg2::Line { a: origin + Vec2::new(x0 * w, y0 * h), b: origin + Vec2::new(x1 * w, y1 * h) }, width, false))
    };
    match lay {
        Lay::Parallel => {
            line(0.0, 0.25, 1.0, 0.25);
            line(0.0, 0.75, 1.0, 0.75);
        }
        Lay::Perpendicular => {
            line(0.0, 0.0, 1.0, 0.0);
            line(0.5, 0.0, 0.5, 1.0);
        }
        Lay::Crossed => {
            line(0.0, 0.0, 1.0, 1.0);
            line(0.0, 1.0, 1.0, 0.0);
        }
        Lay::Multidirectional => {
            line(0.0, 0.0, 0.0, 1.0);
            line(0.0, 1.0, 0.5, 0.0);
            line(0.5, 0.0, 1.0, 1.0);
            line(1.0, 1.0, 1.0, 0.0);
        }
        Lay::Circular => strokes.push(stroke(
            Seg2::Arc { center: origin + Vec2::new(0.5 * w, 0.5 * h), radius: 0.5 * h, start: PI / 4.0, sweep: 1.5 * PI },
            width,
            false,
        )),
        Lay::Radial => {
            line(0.0, 0.0, 0.0, 1.0);
            line(0.0, 1.0, 0.5, 1.0);
            line(0.5, 0.5, 0.0, 0.5);
            line(0.0, 0.5, 1.0, 0.0);
            strokes.push(stroke(
                Seg2::Arc { center: origin + Vec2::new(0.5 * w, 0.75 * h), radius: 0.25 * h, start: -PI / 2.0, sweep: PI },
                width,
                false,
            ));
        }
        Lay::Particulate => {
            line(0.0, 0.0, 0.0, 1.0);
            line(0.0, 1.0, 0.5, 1.0);
            line(0.5, 0.5, 0.0, 0.5);
            strokes.push(stroke(
                Seg2::Arc { center: origin + Vec2::new(0.5 * w, 0.75 * h), radius: 0.25 * h, start: -PI / 2.0, sweep: PI },
                width,
                false,
            ));
        }
    }
    strokes
}

#[cfg(test)]
#[path = "surface_texture_tests.rs"]
mod tests;
