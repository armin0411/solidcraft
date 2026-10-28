use super::*;
use crate::paper::{Group, ObjectKind, Scene};

fn fillet(size: f64) -> WeldSide {
    WeldSide { kind: Kind::Fillet, size: Some(Size::Leg(size)), length_mm: Some(50.0), intermittent: None, contour: None }
}

fn note() -> Weld {
    Weld { arrow_side: Some(fillet(5.0)), other_side: None, all_round: false, field: false, tail: None }
}

// Deliberately simple test font; production supplies actual font metrics.
fn metric(text: &str, height: f64) -> f64 {
    text.chars().count() as f64 * height * 0.6
}

fn draw(weld: &Weld, c: Convention) -> Drawing {
    weld.draw(c, Style::default(), metric).unwrap()
}

fn a(line: IdentificationLine) -> Convention {
    Convention::Iso2553_2019A { identification: line }
}

fn lines(d: &Drawing) -> Vec<(Vec2, Vec2)> {
    d.items
        .iter()
        .flat_map(|i| match i {
            Item::Stroke { path, .. } => path.as_slice(),
            _ => &[],
        })
        .filter_map(|s| match s {
            Seg2::Line { a, b } => Some((*a, *b)),
            _ => None,
        })
        .collect()
}

fn label(d: &Drawing, role: TextRole) -> &Text {
    d.text.iter().find(|t| t.role == role).unwrap()
}

#[test]
fn manual_record_text_retains_every_requirement_and_precision() {
    let mut n = note();
    n.arrow_side = Some(WeldSide {
        size: Some(Size::Throat(5.123456789)),
        intermittent: Some(Intermittent { count: None, pitch_mm: 100.0 }),
        contour: Some(Contour::Concave),
        ..fillet(5.0)
    });
    n.other_side = n.arrow_side.clone();
    n.field = true;
    n.tail = Some("135 / WPS-12 / \"inspect\"".into());
    assert_eq!(
        n.text().unwrap(),
        "arrow: fillet, throat=5.123456789 mm, length=50 mm, chain pitch=100 mm, concave; other: fillet, throat=5.123456789 mm, length=50 mm, chain pitch=100 mm, concave; field; tail=\"135 / WPS-12 / \\\"inspect\\\"\""
    );
    n.arrow_side.as_mut().unwrap().intermittent = None;
    n.other_side.as_mut().unwrap().intermittent = None;
    n.all_round = true;
    assert!(n.text().unwrap().contains("all round; field"));
}

#[test]
fn one_sided_intermittent_notes_do_not_claim_a_chain() {
    for side in [Side::Arrow, Side::Other] {
        let mut n = note();
        n.arrow_side.as_mut().unwrap().intermittent = Some(Intermittent { count: Some(3), pitch_mm: 100.0 });
        if side == Side::Other {
            n.other_side = n.arrow_side.take();
        }
        let summary = n.text().unwrap();
        assert!(summary.contains("intermittent count=3, intermittent pitch=100 mm"), "{summary}");
        assert!(!summary.contains("chain"), "{summary}");
        let spec = n.arrow_side.as_ref().or(n.other_side.as_ref()).unwrap().clone();
        n.arrow_side = Some(spec.clone());
        n.other_side = Some(spec);
        assert!(n.text().unwrap().contains("chain count=3, chain pitch=100 mm"));
    }
}

#[test]
fn round_trip_and_unknown_fields() {
    let n = note();
    let json = serde_json::to_string(&n).unwrap();
    let read: Weld = serde_json::from_str(&json).unwrap();
    read.validate().unwrap();
    assert_eq!(read, n);
    let mut value = serde_json::to_value(n).unwrap();
    value["arrow_side"]["silent_typo"] = true.into();
    assert!(serde_json::from_value::<Weld>(value).is_err());
    for c in [a(IdentificationLine::Above), a(IdentificationLine::Below), Convention::Iso2553_2019B, Convention::AwsA2_4_2020] {
        assert_eq!(serde_json::from_str::<Convention>(&serde_json::to_string(&c).unwrap()).unwrap(), c);
    }
    assert!(serde_json::from_str::<Kind>("\"single_u\"").is_err());
    assert!(serde_json::from_str::<Size>(r#"{"kind":"leg","mm":5,"surprise":2}"#).is_err());
    let unit = serde_json::to_value(Convention::Iso2553_2019B).unwrap();
    let mut mixed = serde_json::Map::new();
    mixed.insert(unit.as_str().unwrap().into(), serde_json::json!({"identification": "above"}));
    assert!(serde_json::from_value::<Convention>(serde_json::Value::Object(mixed)).is_err());
    let mut mixed = serde_json::to_value(a(IdentificationLine::Above)).unwrap();
    mixed.as_object_mut().unwrap().values_mut().next().unwrap()["surprise"] = true.into();
    assert!(serde_json::from_value::<Convention>(mixed).is_err());
}

#[test]
fn hostile_dimensions_are_rejected_by_validation_text_and_drawing() {
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0, 1e-7, MAX_DIMENSION_MM + 1.0] {
        for field in 0..3 {
            let mut n = note();
            let s = n.arrow_side.as_mut().unwrap();
            match field {
                0 => s.size = Some(Size::Leg(v)),
                1 => s.length_mm = Some(v),
                _ => s.intermittent = Some(Intermittent { count: None, pitch_mm: v }),
            }
            assert!(n.validate().is_err(), "{v} field {field}");
            assert!(n.text().is_err());
            assert!(n.draw(Convention::AwsA2_4_2020, Style::default(), metric).is_err());
        }
    }
    for v in [MIN_DIMENSION_MM, MAX_DIMENSION_MM] {
        let mut n = note();
        let s = n.arrow_side.as_mut().unwrap();
        s.size = Some(Size::Leg(v));
        s.length_mm = Some(v);
        assert!(n.validate().is_ok());
        assert!(draw(&n, Convention::AwsA2_4_2020).text.iter().any(|t| t.text == v.to_string()));
    }
}

#[test]
fn invalid_size_kind_combinations_fail_and_supported_ones_survive() {
    for kind in [Kind::Fillet, Kind::SquareButt, Kind::SingleVButt] {
        for size in [Size::Leg(5.0), Size::Throat(5.0), Size::DeepPenetrationThroat(5.0), Size::ButtPenetration(5.0)] {
            let s = WeldSide { kind, size: Some(size), ..fillet(5.0) };
            let should_pass = (kind == Kind::Fillet) != matches!(size, Size::ButtPenetration(_));
            assert_eq!(s.validate().is_ok(), should_pass);
        }
    }
}

#[test]
fn pitch_is_centre_spacing_and_must_leave_a_real_gap() {
    for pitch in [49.0, 50.0, 50.0 + 1e-7] {
        let s = WeldSide { intermittent: Some(Intermittent { count: None, pitch_mm: pitch }), ..fillet(5.0) };
        assert!(s.validate().is_err());
    }
    for length in [MIN_DIMENSION_MM, 50.0] {
        let s = WeldSide {
            length_mm: Some(length),
            intermittent: Some(Intermittent { count: None, pitch_mm: length + MIN_DIMENSION_MM }),
            ..fillet(5.0)
        };
        assert!(s.validate().is_ok());
    }
    let mut n = note();
    n.arrow_side.as_mut().unwrap().intermittent = Some(Intermittent { count: None, pitch_mm: 100.0 });
    assert!(n.validate().is_ok());
    assert_eq!(label(&draw(&n, Convention::AwsA2_4_2020), TextRole::Length(Side::Arrow)).text, "50-100");
    assert!(n.draw(Convention::Iso2553_2019B, Style::default(), metric).is_err());
    n.arrow_side.as_mut().unwrap().intermittent.as_mut().unwrap().count = Some(3);
    assert_eq!(label(&draw(&n, Convention::Iso2553_2019B), TextRole::Length(Side::Arrow)).text, "3×50 (50)");
    assert!(n.text().unwrap().contains("intermittent count=3, intermittent pitch=100 mm"));
    assert!(n.draw(Convention::AwsA2_4_2020, Style::default(), metric).is_err());
    n.arrow_side.as_mut().unwrap().intermittent.as_mut().unwrap().count = Some(1);
    assert!(n.validate().is_err());
    n.arrow_side.as_mut().unwrap().intermittent.as_mut().unwrap().count = Some(u16::MAX);
    assert!(n.validate().is_ok());
    n.arrow_side.as_mut().unwrap().length_mm = None;
    assert!(n.validate().is_err());
}

#[test]
fn contradictory_round_and_intermittent_requirements_are_rejected() {
    let mut n = note();
    n.arrow_side.as_mut().unwrap().intermittent = Some(Intermittent { count: None, pitch_mm: 100.0 });
    n.all_round = true;
    assert!(n.validate().is_err());
    n.all_round = false;
    n.other_side = Some(fillet(5.0));
    assert!(n.validate().is_err());
    n.other_side = n.arrow_side.clone();
    assert!(n.validate().is_ok());
    n.arrow_side.as_mut().unwrap().intermittent.as_mut().unwrap().count = Some(3);
    assert!(n.validate().is_err()); // One side cannot silently supply the other side's count.
    n.other_side.as_mut().unwrap().intermittent.as_mut().unwrap().count = Some(4);
    assert!(n.validate().is_err());
    n.other_side.as_mut().unwrap().intermittent.as_mut().unwrap().count = Some(3);
    assert!(n.validate().is_ok());
    n.other_side.as_mut().unwrap().length_mm = Some(25.0);
    assert!(n.validate().is_err());
}

#[test]
fn tail_only_notes_work_without_any_bead_or_dashed_line() {
    let mut n = Weld { arrow_side: None, other_side: None, all_round: false, field: false, tail: Some("WPS-135".into()) };
    let drawing = draw(&n, a(IdentificationLine::Above));
    assert_eq!(drawing.items.len(), 2); // Reference and fork; the leader belongs to W19.
    assert_eq!(drawing.text.len(), 1);
    assert_eq!(label(&drawing, TextRole::Tail).text, "WPS-135");
    n.tail = None;
    assert!(n.validate().is_err());
    n.field = true;
    n.all_round = true;
    assert!(n.validate().is_err());
}

#[test]
fn tail_limits_use_bytes_and_reject_visual_control_injection() {
    let mut n = note();
    for text in ["", " \t", "x\ny", "x\ry", "x\0y", "x\u{202e}y", "x\u{2067}y", "x\u{2028}y"] {
        n.tail = Some(text.into());
        assert!(n.validate().is_err(), "{text:?}");
    }
    n.tail = Some("é".repeat(MAX_TAIL_BYTES / 2));
    n.validate().unwrap();
    n.tail.as_mut().unwrap().push('é');
    assert!(n.validate().is_err());
    n.tail = Some("135 / WPS-α / inspect <&>".into());
    n.validate().unwrap();
}

#[test]
fn iso_a_side_meaning_follows_line_identity_in_both_positions() {
    let mut n = note();
    n.other_side = Some(fillet(7.0));
    let above = draw(&n, a(IdentificationLine::Above));
    let below = draw(&n, a(IdentificationLine::Below));
    let h = Style::default().height_mm;
    for (drawing, sign) in [(&above, 1.0), (&below, -1.0)] {
        let arrow = label(drawing, TextRole::Size(Side::Arrow));
        let other = label(drawing, TextRole::Size(Side::Other));
        assert_eq!(arrow.text, "z5");
        assert_eq!(other.text, "z7");
        assert!(sign * arrow.baseline.y < 0.0);
        assert!(sign * other.baseline.y > 2.0 * h);
        assert!(lines(drawing).iter().any(|(a, b)| a.y == sign * 2.0 * h && b.y == a.y));
    }
    assert_eq!(n.arrow_side.unwrap().size, Some(Size::Leg(5.0)));
}

#[test]
fn iso_b_and_aws_arrow_side_is_below_and_swapping_moves_content() {
    let mut n = note();
    n.other_side = Some(fillet(7.0));
    for c in [Convention::Iso2553_2019B, Convention::AwsA2_4_2020] {
        let first = draw(&n, c);
        assert!(label(&first, TextRole::Size(Side::Arrow)).baseline.y < 0.0);
        assert!(label(&first, TextRole::Size(Side::Other)).baseline.y > 0.0);
        assert_eq!(lines(&first).iter().filter(|(a, b)| a.y == b.y).count(), 1);
        let mut swapped = n.clone();
        std::mem::swap(&mut swapped.arrow_side, &mut swapped.other_side);
        let second = draw(&swapped, c);
        assert_eq!(label(&first, TextRole::Size(Side::Arrow)).text, label(&second, TextRole::Size(Side::Other)).text);
        assert_eq!(label(&first, TextRole::Size(Side::Other)).text, label(&second, TextRole::Size(Side::Arrow)).text);
    }
}

#[test]
fn symmetric_iso_a_omits_identification_only_when_all_side_data_matches() {
    let mut n = note();
    n.other_side = n.arrow_side.clone();
    assert_eq!(draw(&n, a(IdentificationLine::Above)).items, draw(&n, a(IdentificationLine::Below)).items);
    for pos in [IdentificationLine::Above, IdentificationLine::Below] {
        let d = draw(&n, a(pos));
        assert_eq!(lines(&d).iter().filter(|(a, b)| a.y == b.y).count(), 1);
        assert!(label(&d, TextRole::Size(Side::Arrow)).baseline.y < 0.0);
        assert!(label(&d, TextRole::Size(Side::Other)).baseline.y > 0.0);
    }
    n.other_side.as_mut().unwrap().contour = Some(Contour::Convex);
    assert!(lines(&draw(&n, a(IdentificationLine::Above))).iter().filter(|(a, b)| a.y == b.y).count() > 1);
}

#[test]
fn size_prefixes_are_not_silently_converted_to_aws_legs() {
    for (size, text) in [(Size::Leg(5.0), "z5"), (Size::Throat(5.0), "a5"), (Size::DeepPenetrationThroat(5.0), "s5")] {
        let mut n = note();
        n.arrow_side.as_mut().unwrap().size = Some(size);
        let before = n.clone();
        assert_eq!(label(&draw(&n, Convention::Iso2553_2019B), TextRole::Size(Side::Arrow)).text, text);
        assert_eq!(n.draw(Convention::AwsA2_4_2020, Style::default(), metric).is_ok(), matches!(size, Size::Leg(_)));
        assert_eq!(n, before);
    }
    let mut n = note();
    n.arrow_side.as_mut().unwrap().kind = Kind::SingleVButt;
    n.arrow_side.as_mut().unwrap().size = Some(Size::ButtPenetration(5.0));
    assert_eq!(label(&draw(&n, Convention::Iso2553_2019B), TextRole::Size(Side::Arrow)).text, "s5");
    assert!(n.draw(Convention::AwsA2_4_2020, Style::default(), metric).is_err());
}

#[test]
fn elementary_shapes_have_analytic_coordinates_and_keep_the_vertical_leg_left() {
    let h = 3.5;
    let mut items = Vec::new();
    elementary(&mut items, Kind::Fillet, 10.0, 0.0, -h, 0.35);
    assert_eq!(items, vec![Item::Stroke { path: vec![line(10.0, 0.0, 10.0, -3.5), line(10.0, -3.5, 13.5, 0.0)], closed: false, width_mm: 0.35 }]);
    items.clear();
    elementary(&mut items, Kind::SingleVButt, 10.0, 0.0, h, 0.35);
    let top = h * 3.0_f64.sqrt() / 2.0;
    assert_eq!(items, vec![Item::Stroke { path: vec![line(10.0, top, 11.75, 0.0), line(11.75, 0.0, 13.5, top)], closed: false, width_mm: 0.35 }]);
    items.clear();
    elementary(&mut items, Kind::SquareButt, 10.0, 0.0, h, 0.35);
    assert_eq!(items.len(), 2);
    for i in items {
        let Item::Stroke { path, .. } = i else { panic!() };
        assert_eq!(path[0].start().y, 0.0);
        assert_eq!(path[0].end().y, 3.5);
    }
}

#[test]
fn round_field_and_tail_marks_are_at_reference_endpoints() {
    let mut n = note();
    n.all_round = true;
    n.field = true;
    n.tail = Some("WPS-12".into());
    let d = draw(&n, Convention::AwsA2_4_2020);
    let h = Style::default().height_mm;
    assert!(d.items.iter().any(|i| matches!(i, Item::Stroke { path, closed: true, .. } if matches!(path.as_slice(), [Seg2::Arc {center,radius,..}] if *center == Vec2::ZERO && *radius == 0.3*h))));
    assert!(lines(&d).contains(&(Vec2::new(0.0, 0.3 * h), Vec2::new(0.0, 2.5 * h))));
    assert!(lines(&d).contains(&(Vec2::new(0.3 * h, 0.0), Vec2::new(d.reference_length_mm, 0.0))));
    assert!(lines(&d).iter().any(|(a, b)| *a == Vec2::new(d.reference_length_mm, 0.0) && *b == Vec2::new(d.reference_length_mm + h, h)));
    assert!(label(&d, TextRole::Tail).baseline.x > d.reference_length_mm + h);
}

#[test]
fn contours_bulge_outward_or_inward_on_each_side() {
    for contour in [Contour::Convex, Contour::Concave] {
        let mut n = note();
        n.arrow_side.as_mut().unwrap().contour = Some(contour);
        n.other_side = n.arrow_side.clone();
        let d = draw(&n, Convention::AwsA2_4_2020);
        let curves: Vec<_> = d
            .items
            .iter()
            .flat_map(|i| match i {
                Item::Stroke { path, .. } => path.as_slice(),
                _ => &[],
            })
            .filter_map(|p| match p {
                Seg2::Cubic { p0, p1, p3, .. } => Some((p0, p1, p3)),
                _ => None,
            })
            .collect();
        assert_eq!(curves.len(), 2);
        for (start, control, end) in curves {
            let chord_control = *start + (*end - *start) / 3.0;
            assert_eq!(start.y.signum() * (control.y - chord_control.y) > 0.0, contour == Contour::Convex);
        }
    }
}

#[test]
fn concave_fillet_keeps_ink_clear_at_the_widest_stroke() {
    let mut n = note();
    n.arrow_side.as_mut().unwrap().contour = Some(Contour::Concave);
    let d = n.draw(Convention::AwsA2_4_2020, Style { height_mm: 1.0, width_mm: 0.25 }, metric).unwrap();
    let x = label(&d, TextRole::Size(Side::Arrow)).baseline.x + label(&d, TextRole::Size(Side::Arrow)).width_mm + 1.0;
    let curve = d
        .items
        .iter()
        .find_map(|i| match i {
            Item::Stroke { path, .. } => path.iter().find(|s| matches!(s, Seg2::Cubic { .. })),
            _ => None,
        })
        .unwrap();
    let Seg2::Cubic { p0, p1, p2, p3 } = curve else { panic!() };
    let midpoint = *p0 * 0.125 + *p1 * 0.375 + *p2 * 0.375 + *p3 * 0.125;
    // Normal distance to the fillet's diagonal must exceed the sum of the two half widths.
    assert!((midpoint.x - x + midpoint.y.abs() - 1.0) / 2.0_f64.sqrt() > 0.25);
}

#[test]
fn geometry_validates_as_paper_for_all_supported_shapes_sizes_and_sides() {
    for c in [a(IdentificationLine::Above), a(IdentificationLine::Below), Convention::Iso2553_2019B, Convention::AwsA2_4_2020] {
        for kind in [Kind::Fillet, Kind::SquareButt, Kind::SingleVButt] {
            for contour in [None, Some(Contour::Flush), Some(Contour::Concave), Some(Contour::Convex)] {
                for h in [1.0, 3.5, 20.0] {
                    let side = WeldSide { kind, size: None, contour, ..fillet(5.0) };
                    for (arrow, other) in [(Some(side.clone()), None), (None, Some(side.clone())), (Some(side.clone()), Some(side.clone()))] {
                        let n = Weld { arrow_side: arrow, other_side: other, all_round: true, field: true, tail: Some("WPS-135".into()) };
                        let d = n.draw(c, Style { height_mm: h, width_mm: h / 10.0 }, metric).unwrap();
                        Scene { size_mm: Vec2::new(420.0, 297.0), groups: vec![Group { object: 1, kind: ObjectKind::TitleBlock, items: d.items }] }
                            .validate()
                            .unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn physical_style_scales_geometry_and_metrics_together() {
    let n = note();
    let one = draw(&n, Convention::AwsA2_4_2020);
    let two = n.draw(Convention::AwsA2_4_2020, Style { height_mm: 7.0, width_mm: 0.7 }, metric).unwrap();
    assert_eq!(two.reference_length_mm, 2.0 * one.reference_length_mm);
    for (a, b) in lines(&one).iter().zip(lines(&two)) {
        assert_eq!(b.0, a.0 * 2.0);
        assert_eq!(b.1, a.1 * 2.0);
    }
    for (a, b) in one.text.iter().zip(&two.text) {
        assert_eq!(a.text, b.text);
        assert_eq!(b.baseline, a.baseline * 2.0);
    }
}

#[test]
fn actual_text_metrics_reserve_space_without_touching_the_symbol_or_tail() {
    let mut n = note();
    n.tail = Some("WPS".into());
    let d = n.draw(Convention::AwsA2_4_2020, Style::default(), |_, _| 1000.0).unwrap();
    let size = label(&d, TextRole::Size(Side::Arrow));
    let length = label(&d, TextRole::Length(Side::Arrow));
    assert!(size.baseline.x + size.width_mm < length.baseline.x - 3.5);
    assert!(length.baseline.x + length.width_mm < d.reference_length_mm);
    assert!(d.items.len() < 500);
    let d = n.draw(a(IdentificationLine::Above), Style { height_mm: 1.0, width_mm: 0.1 }, |_, _| 1000.0).unwrap();
    assert!(d.items.len() < 500);
    assert!(lines(&d).iter().filter(|(a, b)| a.y == 2.0 && b.y == 2.0).all(|(a, _)| a.x >= 2.0));
}

#[test]
fn hostile_styles_and_metrics_fail_without_partial_output() {
    let n = note();
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 0.99, 20.01] {
        assert!(n.draw(Convention::AwsA2_4_2020, Style { height_mm: value, width_mm: 0.1 }, metric).is_err());
    }
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 0.009, 1.0] {
        assert!(n.draw(Convention::AwsA2_4_2020, Style { height_mm: 3.5, width_mm: value }, metric).is_err());
    }
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 1000.1] {
        assert!(n.draw(Convention::AwsA2_4_2020, Style::default(), |_, _| value).is_err());
    }
}
