# GD&T records and original symbols (W28a, #7257)

`solidcraft_geom::gdt` supplies single-row feature-control frames and boxed datum
identifiers as data, text transcriptions and local vector geometry. It depends
only on the existing geometry, serde and error types, with no UI, operating-system
API, font or new dependency. It can be used on wasm.

A caller explicitly selects `iso1101_2017_iso5459_2024` or `asme_y14_5_2018`.
These select the ISO 1101:2017 / ISO 5459:2024 pair or ASME Y14.5-2018 (R2024).
The family and edition travel with each record; conversion is never automatic.

## Supported grammar

- ISO's fourteen characteristics; ASME's twelve, rejecting concentricity and
  symmetry under the supported 2018 edition.
- One tolerance compartment: width, diameter or spherical diameter, a value in
  millimetres, optional maximum/least material modifier and optional projected
  height. No arbitrary symbol strings or mutually conflicting material modifiers.
- A surface or derived feature of size is explicitly recorded. Roundness,
  cylindricity, profiles and runout control surfaces in this subset. Position,
  concentricity/coaxiality and symmetry control derived size features here.
- Width is supported for surfaces; diameter for derived straightness, orientation,
  position and ISO concentricity/coaxiality; spherical diameter only for position.
  Flatness and symmetry use width. The supported ISO concentricity/coaxiality
  indication uses diameter. These are subset restrictions, not a complete account
  of every standard's toleranced-feature and zone combination.
- Maximum/least material on the tolerance requires a derived size feature and
  straightness, flatness, orientation or position. ISO means material requirement;
  ASME means material condition. On datum references, ASME means material boundary;
  ISO retains material-requirement semantics (whose evaluation is ISO 2692 work).
  An omitted modifier retains the selected family's default. No circled S is used.
- Positive tolerance/height values range from 1e-9 to 10000 mm. Zero tolerance is
  supported at maximum material only. These bounds limit resources and text size;
  they are not normative limits. Negative zero transcribes as zero.
- An in-frame projected height is supported for ASME position with a width or
  diameter zone. ISO's external projected-height dimension is not treated as an
  ASME in-frame height: this field is rejected under ISO. ISO projected zones
  await the dimension/attachment work.
- Zero to three ordered, single-datum references. Form controls take no datums;
  orientation, runout, concentricity and symmetry require one to three. Position
  and profiles can be datum-free. Datum-free position is accepted and returns
  `DatumFreePositionNeedsPatternContext` for the later drawing checker, following
  the plan's instruction not to reject all datum-free position specifications.
- Datum identifiers are one or two uppercase ASCII letters excluding I, O and Q.
  This is a conservative supported alphabet, not a claim that ISO excludes all
  three letters: the study's ISO 5459:2024 preview found no exclusion, whereas
  its proposed command excludes them. Ordered references cannot repeat an
  identifier. Common/multiple-feature datums
  such as A-B require a future structured grammar and are rejected here.

Every text/layout boundary revalidates the record, including deserialized input.
Unknown serde fields and enum variants are errors. Encoded-input callers must
still bound bytes and nesting before deserialization. Validation does not establish
that cited datums exist, are unique within the part, are size features when a
material modifier requires that, or constrain the intended degrees of freedom.
Those checks need a document/model context absent from this foundation slice.

## Geometry and text

`Frame::compartments` returns reading-order cells. `Frame::text` joins them with
` | `, for example `⌖ | ⌀0.1Ⓜ | A | B | C`. This is a transcription, not a
parseable interchange format. Values retain their shortest round-trip decimal
form in millimetres, without precision rounding or locale substitution.
`DatumIdentifier::text` returns `[A]` for A.

`layout` returns paper `Item` strokes/fills, size and compartment bounds, in
millimetres from the frame's local lower-left corner. It does not create a paper
scene group or choose a layer. Existing writers can consume these primitives once
the sheet annotation layer groups and places them. No view scale enters the API.
A datum layout is its boxed label only; its connecting line and triangle belong
to the deferred leader/attachment layer.

All geometry is original code: lines, analytic circular arcs and filled runout
arrowheads, plus a small original single-stroke alphabet for values and datum
letters. Characteristic and modifier Unicode characters are transcriptions only;
no font glyph is used to draw a symbol. No standard's figures, fonts, text or
commercial CAD assets/code were copied.

Lettering height h is 1.8–20 mm, stroke width h/10, frame height 2h, first cell
square, following cells content-sized with horizontal ink clearance of at least
max(2h/10,0.7 mm). Padding additionally allows for the frame's half stroke width
and the lettering's worst permitted miter extension, so the clearance applies to
visible ink rather than centre lines.
A single-letter datum label is square. Bounds describe stroke centre lines;
painted ink extends h/20 outside the outer frame. The public [SIST preview of ISO 7083:2021](https://preview.sist.si/sist-preview/66740/2848daeb646f4ca9b325c29dd16e96b1/ISO-7083-2021.pdf),
Table 2 and clause 5, independently confirms the Type B frame/lettering/line-width
ratios, square first compartment and horizontal clearance used here. The custom
1.8 mm height is a supported layout policy, outside that table's recommended sizes.

For the ISO diameter prefix, Table 6 no. 32 supplies the construction grid. With
d=h/10, the circle's centre-line diameter is 6d and the diagonal spans 3d
horizontally and 9d vertically. Analytic filled circles of radius d/2 give its
round ends without changing paper's butt-cap stroke contract. Its painted width
is 7d and height is 10d. These are mathematical proportions used in original code;
no standard artwork is extracted or shipped. The ASME diameter construction stays
separate and provisional pending review, rather than inheriting ISO proportions.
At h=3.5 mm, the example `⌀0.1Ⓜ` compartment is 17.5 mm wide under ISO and 18.2 mm
in the ASME preview; the five-cell ISO example is 40.25 × 7 mm.

**The characteristic and circled-modifier outlines in ISO 1101 Annex F / ISO 7083
and the ISO 5459:2024 datum feature indicator have not been independently
verified.** The original stroke alphabet is also not a verified ISO 3098 alphabet.
The preview stops before their construction tables. The study also
identifies the 2024 datum feature indicator as changed. The local boxed label avoids inventing that deferred indicator.
Drafting review against the licensed current standards is still required before
claiming normative graphical conformance or manufacturing completeness.

## Evidence and boundaries

Requirements: [issue #7257](https://github.com/tangency-labs/solidcraft/issues/7257),
[drawings study](https://github.com/tangency-labs/solidcraft/issues/860#issuecomment-6216908058)
§5.4.0, §5.4.3–4, §11.1, §11.8–9 and §11.26–28, and
[plan W28](https://github.com/tangency-labs/solidcraft/issues/860#issuecomment-6234779815).
The issue narrows the larger W28 row to this below-UI foundation.

Publisher scope/edition references:
[ISO 1101:2017](https://www.iso.org/standard/66777.html),
[ISO 5459:2024](https://www.iso.org/standard/87855.html),
[ISO 7083:2021](https://www.iso.org/standard/66740.html) and
[ASME Y14.5-2018 (R2024)](https://www.asme.org/codes-standards/find-codes-standards/y14-5-dimensioning-tolerancing).
These public catalogue descriptions confirm editions and subject, not every
syntax or numeric proportion. The study supplies the working rule evidence;
unverified details remain explicitly limited above.

No sheet placement, leader, view/dimension attachment, datum target, composite or
stacked frame, model evaluation, document persistence or UI/engine command is
introduced. Tangent-plane/free-state modifiers, unequal profiles, common datums,
auxiliary plane indicators and ISO 5459:2024's advanced datum operators also await
separate validated grammar. Unknown variants fail rather than being approximated.
