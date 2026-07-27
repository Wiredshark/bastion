# APEX vector/seed pin provenance

Per Fable's ruling: do not overwrite the master build order's printed pins.
Annotate each affected artifact with (guide-printed pin) + (this repo's
committed-fixture pin) + a normalization check, so provenance is preserved
and future SHA gates don't cry wolf on line-ending/BOM noise alone. The
upstream fix (correcting the guide's printed pins, or re-exporting the
Drive originals cleanly) was originally routed to the guide's author via
Ben.

**Terminal update, 2026-07-26:** Ben reports the ChatGPT-side artifacts
that this program's unresolved items were routed upstream through were
hallucinated — there is no real recovered content coming for any of them.
Every item below still marked "routed to the guide author" or "PARTIALLY
RESOLVED" pending upstream input has been converted to a final
disposition (see each section); nothing in what this repo already trusts
changes, only the provenance labeling of what was never real.

**Correction to Builder Opus 5's initial framing:** Opus 5 reported "the
Drive vector file is BOM+CRLF; after normalization, content-identical, 0
diffs" for all three drifted artifacts, generalizing from the T0.2 case. I
independently re-derived all three before writing this doc (never
propagate an unverified claim into a permanent record) — the BOM-strip +
CRLF→LF + trailing-newline normalization **only fully reconciles one of
the three** with its guide-printed pin. The other two remain genuinely
unexplained by pure formatting and need the guide author's attention, not
just a normalization footnote.

## `PROJECT-BASTION-APEX-BOUNDARY-INVENTORY-SEED-v1.csv` (`APEX-T0.1`)

```
guide-printed pin:  8a44b30fd4c61778c39f91c439263ada0a9807a48c4fe000a274e7c8053a8287
raw Drive bytes:    a5c2ccf3babbbfb5592ee379e4e1efe50b6ebd61a820481e0aa02a45c238d383
normalized bytes:   8a44b30fd4c61778c39f91c439263ada0a9807a48c4fe000a274e7c8053a8287  <-- MATCHES guide pin
this repo's fixture: readme/apex/APEX-BOUNDARY-INVENTORY-SEED-v1.csv (re-derived table, not a byte copy of the Drive CSV)
```

**RESOLVED**: normalization (strip UTF-8 BOM, CRLF→LF, trim trailing
whitespace to one final newline) makes the raw Drive file byte-identical to
the guide's printed pin. The mismatch was pure export-path formatting, not
content drift. No further action needed on this one.

## `PROJECT-BASTION-APEX-MANIFEST-CBOR-GOLDEN-VECTORS-v1.json` (`APEX-T0.2`)

```
guide-printed pin:  8aba6c9ba899fb761d0085d3e711f1f4f423c90948f7d5bcc2d377c0dc84eaa7
raw Drive bytes:    4e9b9c9503deeda6ed64f6c883b576218cb9a2e3f303131408785fb18f4bd340
normalized bytes:   0dcda3aef232a734c9d57be2252dfe5ae1f471aecb5805cbd0fe8f313a7b3a8e  <-- does NOT match guide pin
this repo's fixture (common/tests/fixtures/apex_manifest_v1/golden-vectors.json): 0dcda3aef232a734c9d57be2252dfe5ae1f471aecb5805cbd0fe8f313a7b3a8e  <-- MATCHES normalized Drive bytes exactly
```

**RESOLVED (terminal, 2026-07-26)**: this repo's committed test fixture was
already confirmed byte-identical to the current Drive file after the same
BOM/CRLF/trailing-whitespace normalization — so this repo never introduced
content drift of its own. The remaining open question was why that
normalized content (`0dcda3ae...`) still didn't match the guide's printed
pin (`8aba6c9b...`). Ben confirmed there is no real Drive-side revision
this printed pin could correspond to — it was a hallucinated value from the
same routing, not a stale-but-real export. Disposition: the
**guide-printed pin `8aba6c9b...` is CONFIRMED_FABRICATED**; **this repo's
normalized-content pin `0dcda3ae...` is AUTHORITATIVE**. The delivered
38-vector set itself does not change — it was already independently
established as correct via round-trip through the real T0.2 encoder/decoder
(`common/tests/apex_manifest_encoding_v1.rs`), which never depended on
matching the fabricated pin. Only the provenance label changes: from "an
unexplained guide/repo pin mismatch, flagged for the guide author" to "the
guide's pin was never real; this repo's pin was correct the whole time."

## `PROJECT-BASTION-APEX-PROGRAM-REGISTRY-SEED-v1.json` (`APEX-A.3`)

```
guide-printed pin:  e394ff1c4ccf4ff211b0b2d4603837d573d496bf0a886aa137c9b59096ec4085
raw Drive bytes:    e394ff1c4ccf4ff211b0b2d4603837d573d496bf0a886aa137c9b59096ec4085  <-- MATCHES guide pin already
normalized bytes:   3ef22ba70c957c5cb21c8f71e6e8a2d68b6339345fcb4f57590e639c764a4ff8  <-- does NOT match (normalization changed a byte-identical file!)
```

**DIFFERENT SITUATION, NOT A MISMATCH**: the raw Drive file already matches
the guide's printed pin exactly, byte-for-byte, with no normalization
needed. (The A.3 packet's own text already noted this as "artifact-version
drift... not a source-code contradiction" between two *prose copies* of
the digest, not between the guide pin and the Drive file itself.) Recorded
here only so a future reader doesn't misapply the T0.1/T0.2 normalization
pattern to this file and get confused by the fact that normalizing an
already-clean file changes its hash.

## Filename alias table

Three master-build-order citations name a file that does not exist under
that exact name on Drive. Recorded here (not silently substituted) so a
future reader searching for the cited name doesn't conclude the file is
missing entirely:

| Cited in guide | Actual Drive filename | Status |
|---|---|---|
| `PROJECT-BASTION-APEX-MANIFEST-CODEC-GOLDEN-VECTORS-v1.json` (`APEX-T0.2`) | `PROJECT-BASTION-APEX-MANIFEST-CBOR-GOLDEN-VECTORS-v1.json` | **AUTHORITATIVE (terminal, 2026-07-26).** No upstream ratification of the guide's citation is coming; the Drive filename is the locally-canonical name for this program going forward, and remains what this program's T0.2 work actually uses. |
| `PROJECT-BASTION-APEX-DIGEST-CONTENT-GOLDEN-VECTORS-v1.json` (`APEX-T0.3`) | `PROJECT-BASTION-APEX-DIGEST-GOLDEN-VECTORS-v1.json` | **AUTHORITATIVE (terminal, 2026-07-26).** Same disposition as the T0.2 row above — locally-canonical, no upstream ratification pending. |
| `PROJECT-BASTION-APEX-T0.1-SCALAR-GOLDEN-VECTORS-v1.json` (`APEX-T0.1`) | *(none)* | **NEVER_EXISTED (terminal, 2026-07-26).** No file matches this name or any close variant, and Ben confirmed no recovery is coming — the absence flag was correct, not a naming mismatch to fix. `APEX-T0.1`'s own conformance suite (`common/src/apex/scalar.rs`'s `#[cfg(test)]` module, `apex::scalar::tests`, 10 tests covering min/max/transparency/checked-conversion/serde-round-trip boundaries) is the vector authority for `APEX-T0.1` going forward — it was never blocked on this citation and does not depend on it appearing. |

Both filename-alias rows above and the T0.1 absence were originally routed
to the guide's author via Ben pending upstream correction/delivery; as of
2026-07-26 that routing is closed (see the terminal note at the top of this
document) and the dispositions above are final.
