# APEX-A.2 / APEX-A.3 canonical CBOR emission schema v1

Closes the loop Builder Opus 5's Batch-1 review flagged: `APEX-A.2`'s
finding matrix and `APEX-A.3`'s program registry were built before
`APEX-T0.2` (`BastionManifestEncodingV1`) existed, so they used CSV/JSON as
their temporary serialization (per A.1's own "must not invent a competing
canonical codec" principle). Now that T0.2 has landed, both are re-emitted
as canonical CBOR **through the real encoder**
(`common::apex::manifest::encode_value_bytes_v1`), via
`bastion-harness/src/bin/apex_emit_manifest_cbor.rs`. The CSV/JSON files
remain the source of truth and are not deleted; the `.cbor` files are a
byte-exact projection of the same data through the landed codec, with a
round-trip self-check (decode the emitted bytes back and diff against the
source) run as part of emission, not just claimed.

This schema is `NEW-SPEC` — neither A.2 nor A.3's original packets defined
field IDs for a canonical encoding (they predate T0.2), so this document
freezes the mapping this emission tool uses.

**ASCII/non-ASCII fallback (real finding from building this tool):** T0.2's
`MachineTextV1` is ASCII-only by design (V1 identity-text policy). Several
A.2/A.3 prose fields (`problem_group`, `live_observation`, `scope_note`,
...) legitimately contain non-ASCII punctuation (em dashes) inherited from
this program's own writing style. Every `MachineText`-typed field below is
encoded as `MachineText` when its actual content is pure ASCII, and falls
back to a raw `Bytes` value carrying the exact UTF-8 bytes unmodified
otherwise — never silently transliterated or dropped. A decoder must check
the CBOR major type of each such field rather than assuming `MachineText`
unconditionally.

## `APEX-FINDING-STATUS-MATRIX-v1.cbor`

Top-level value: `Array` of finding maps, in the CSV's row order (canonical
guide order). Each finding is a `Map` keyed by:

```
0  finding_id            MachineText
1  problem_group         MachineText
2  status                MachineText  ("OPEN"|"PARTIAL"|"CLOSED"|"SUPERSEDED")
3  live_path              MachineText
4  live_observation        MachineText
5  replacement_rows         Array<MachineText>  (CSV's ";"-separated list, split)
6  scope_note              MachineText
7  live_commit             MachineText  (40 lowercase hex chars)
8  evidence_confidence      MachineText
9  evidence_gap            MachineText, OMITTED when the CSV cell is empty
                            (optional fields are absent map fields, not
                            null -- T0.2 value-model rule)
```

## `APEX-DETERMINISM-PROGRAM-REGISTRY-v1.cbor`

Top-level value: one `Map`:

```
0  schema                    MachineText
1  canonical_guide            MachineText
2  finding_matrix             MachineText
3  audit_basis                MachineText  (40 lowercase hex chars)
4  last_live_commit_checked     MachineText
5  row_order                  Array<MachineText>
6  rows                      Array<Map>            -- see below
7  findings                  Array<Map>            -- see below
8  unresolved_row_references    Array<MachineText>
```

Each row `Map`:

```
0  row_id                    MachineText
1  sequence_index             Unsigned
2  title                     MachineText
3  hard_dependencies           Array<MachineText>
4  finding_ids                Array<MachineText>
5  source_surfaces_status       MachineText
6  packet_file                MachineText, OMITTED when null
7  evidence_status             MachineText
8  rollback_plan_status         MachineText
9  status                     Map { 0=specification, 1=microstep_research,
                               2=implementation, 3=verification,
                               4=deployment -- all MachineText }
```

Each finding `Map`:

```
0  finding_id                MachineText
1  originating_package         MachineText
2  live_status                MachineText
3  closure_rule                Map { 0=kind MachineText,
                               1=row MachineText (Row only, omitted otherwise),
                               2=rows Array<MachineText> (AllOf/AnyOf/SupersededBy only, omitted otherwise),
                               3=rationale MachineText (AnyOf only, omitted otherwise),
                               4=reason MachineText (SupersededBy only, omitted otherwise) }
4  source_anchors               Array<MachineText>
5  last_live_commit_checked       MachineText
```

## Limits used

`max_input_bytes = 4 MiB`, `max_depth = 8`, `max_nodes = 65536`,
`max_array_items = 4096`, `max_map_entries = 32`,
`max_machine_text_bytes = 16384`, `max_byte_string_bytes = 16384` — generous
relative to actual content (largest single field is a few hundred bytes;
largest array is 55 rows), chosen so the emission tool's own decode-and-
diff self-check cannot fail on a legitimate budget rather than a real bug.
