# Welding-symbol records (W30b, #7259)

`solidcraft_geom::weld` provides manual welding requirements, semantic text and original local
symbol geometry. It needs no weldment, bead, model or active document. This is a foundation
below the UI, with no commands, sheet placement, leaders, view attachment or saved drawing
schema yet. It does not implement the model-side weldment work in #4003 or the other symbol
families in #7257 and #7258.

## Contract

`Weld` has independent optional `arrow_side` and `other_side` records, all-round and field
marks, and an optional single-line tail. Each `WeldSide` has a kind, an optional typed size,
nominal length, optional chain-intermittent centre-to-centre pitch and element count, and an optional contour.
Missing dimensions stay unspecified. The record never derives a throat from a leg, infers a
welding process or obtains requirements from visible model geometry.

Supported elementary shapes are fillet, square butt and single-V butt. Fillet sizes explicitly
distinguish leg, throat and deep-penetration throat; butt penetration is a different size kind.
Contours distinguish flush, convex and concave. The tail is author-supplied text, including any
WPS/process/quality references; it is not a validated ISO 4063 or ISO 5817 catalogue. A tail-only
note supports “details elsewhere” without inventing an elementary weld requirement.

Dimensions are finite millimetres in `1e-6..=100000`. Intermittent fillets require a length and
a pitch exceeding it by at least `1e-6 mm`. An optional count is 2 to 65535; it does not allocate
or draw repeated welds. This subset supports chain spacing on two sides
only when their element length, pitch and optional count agree. It rejects all-round intermittent
notes and intermittent butt welds as unsupported combinations. These limits are implementation limits,
not universal manufacturing rules. Tails contain nonblank text of at most 512 UTF-8 bytes;
controls, line separators and bidi overrides/isolates are rejected.

Call `validate()` after serde deserialization or an edit. A file/command boundary must also
bound encoded bytes and nesting before parsing; validation cannot undo parsing allocations.
Unknown fields and unknown enum variants are refused. Every text/geometry entry point
validates again, including for records constructed directly in Rust.

`text()` gives a semantic summary with side names, size meanings, full stored numeric precision
and units, pitch explicitly named as pitch, contours, marks and quoted tail text. It is not a
standard callout, machine interchange format, localized UI label or a statement of compliance.
One-sided elements are described as intermittent; matching elements on both sides are
described as a chain arrangement.

## Drawing and convention choices

`draw(convention, style, measure)` takes an explicit edition/system selection. Changing it
does not modify the semantic record. No drawing-wide default or silent standard conversion
is introduced.

| Convention | Arrow side | Other side | Supported dimension labels |
|---|---|---|---|
| `Iso2553_2019A` | Continuous line | Dashed identification line, explicitly above or below | Fillet `z`, `a`, `s`; butt penetration `s`; continuous length or counted intermittent `n×length (gap)` |
| `Iso2553_2019B` | Below the single line | Above the single line | Same basic ISO labels |
| `AwsA2_4_2020` | Below the single line | Above the single line | Fillet leg without prefix; continuous length or intermittent `length-pitch` |

Identical double-sided ISO A welds omit the identification line and place the elementary
symbols opposite one another at the continuous line. An ISO A tail-only note also has no
identification line. For asymmetric welds, side meaning follows line identity even when the
identification line moves below the continuous line. The fillet's vertical stroke stays left
on either side; changing sides does not mirror the text or turn the symbol sideways.

The returned `Drawing` contains `paper::Item` strokes and semantic `Text` runs with left-aligned
baselines, measured widths and cap heights. Coordinates are local paper mm, x right/y up; the
reference elbow is `(0,0)` and its tail junction is `(reference_length_mm,0)`. No arrow/leader
is drawn. The all-round circle and field flag are at that elbow; the fork and tail are at the
opposite end. The later leader implementation must use these junctions. This fragment alone
is not a complete welding symbol on a sheet.
When composing a sheet later, declare the selected system and the engineering unit (mm).
For an all-round note, reference and flag strokes stop at the circle; a future leader must
also terminate at its boundary instead of running through its hollow interior.

Use the same font and cap height for `measure(text, height_mm)` and for drawing the returned
text. The caller must report unsupported glyphs before accepting the output, and escape text
in its output format. No lettering font, glyph substitution or guessed character-width table
is bundled here. Measured widths must be finite, positive and at most 1000 mm. Symbol height
is `1..=20 mm`; stroke width is `0.01..=height/4`. These bounds cap work and coordinates even
with hostile metrics. Sizes on paper do not change with view scale. Dashes are explicit short
strokes because the current paper scene has no line-pattern primitive.

Geometry is independently constructed from lines, a circular all-round mark and cubic contour
curves. Fillets use a right triangle; square butt uses two parallel strokes; single-V uses
two strokes with a 60-degree included **glyph** angle (not a stored preparation angle).
Fillet contours follow the sloping face; butt contours span the opening. These are original
parametric constructions, not copied standard figures, CAD artwork or font glyphs. They add
no file assets or dependencies.

## Evidence and limits

The [issue](https://github.com/tangency-labs/solidcraft/issues/7259), the linked
[research](https://github.com/tangency-labs/solidcraft/issues/860#issuecomment-6216908058)
(§§5.4.0, 5.4.2, 11.1, 11.8, 11.9, 11.28, 11.32) and the
[W30 plan](https://github.com/tangency-labs/solidcraft/issues/860#issuecomment-6234779815)
define the scope. Public sources checked on 2026-10-26, with vocabulary rechecked on 2026-10-27:

- [ISO's 2553:2019 corrected-edition overview and vocabulary](https://www.iso.org/obp/ui?_escaped_fragment_=iso:std:iso:2553:ed-5:v2:en)
  establishes the dual/single-line systems and semantic distinctions. The lawfully public
  [ISO preview](https://cdn.standards.iteh.ai/samples/72740/745f6baf65c1495fa159e36173dbfd21/ISO-2553-2019.pdf)
  covers elementary symbols and the symmetric double-sided identification-line exception.
- [AWS's own explanation of A2.4:2020 clause 6.1](https://www.aws.org/magazines-and-media/inspection-trends/2025/may/it-may-25-feature-02-symbols/)
  verifies AWS side placement. The publisher's
  [2020 preview](https://pubs.aws.org/Download_PDFS/2__A2_4_2020_PV.pdf) identifies the edition;
  its public [2007 chart](https://app.aws.org/mwf/attachments/64/225364/AWSWeldSymbolchart.pdf)
  supplies the basic `length-pitch`, mark and tail conventions. The publisher's
  [2007 preview](https://pubs.aws.org/Download_PDFS/A2.4_2007PV.pdf), clause 3.1, distinguishes
  an elementary weld mark from the complete welding annotation. Continuity with 2020 beyond
  verified side placement still needs edition-specific drafting review.
- [TWI's authored welding-symbol explanation](https://www.twi-global.com/technical-knowledge/job-knowledge/weld-symbols)
  supports the basic size meanings and ISO line identity. It discusses earlier ISO/EN editions;
  it is not proof of every 2019 clause.
- The [Australian Welding Institute's authored explanation](https://welding.org.au/standards/as-1101-3/)
  distinguishes counted element length and clear spacing, and describes the basic marks and
  contours. Its generic ISO comparison supports `n×length (gap)`; it does not establish all
  edition-specific ISO 2553 clauses or system B rules.

These sources establish the basic subset, not complete current-edition conformity. The
research itself marks several facts and proportions as unverified. Exact edition-specific
symbol proportions, ISO B placement beyond the research's proposed mapping, and the full
dimension grammar still need a drafting reviewer and lawful edition-specific evidence.
The h-based spacing, dash lengths, circle/flag/fork sizes and contour offsets in this module
are explicitly original layout choices pending that review, not asserted normative ratios.

ISO intermittent notation prints clear spacing as `pitch-length`, and requires an explicit
element count. For example, 3 elements each 50 mm long at 100 mm centre pitch produce
`3×50 (50)`; the corresponding uncounted AWS length/pitch label is `50-100`. Missing ISO count
and counted AWS fillet callouts return explicit unsupported errors. Semantic text retains
the original centre pitch and any count in every case. AWS throat/deep-throat/butt-penetration
callouts also return an error instead of converting them to a leg or a different groove
dimension. All output is returned together or an error; no partially drawn or silently
simplified symbol is returned. The ISO notation still needs edition-specific drafting review.

Further work includes alternative intermittent notations/offsets, staggered patterns, additional
weld kinds, compound/multiple-reference symbols, unequal legs, preparation/root dimensions,
finish methods and validated process references. These capabilities are not implied by the
edition identifiers. No standards text, artwork or lettering is redistributed. The issue's
full standards/proportion requirement remains subject to these explicit evidence gaps.

Tests use analytic coordinates, semantic assertions, supported/rejected grammar, line identity,
the symmetric ISO A exception, side swapping, dimension boundaries/non-finite values, hostile
tail text/style/metrics, contour direction, actual-width layout and paper-scene validation.
They establish this implementation's behavior, not standards certification.
