# SERVER FINGERPRINT — **PREREG AMENDMENT: MY PREMISE WAS FALSE**

Amends `SERVER-FINGERPRINT-PREREG.md` (`09106a6559`) **before any code was written**.

## 1 · THE PREMISE, AND ITS REFUTATION

The prereg asserted, twice and emphatically:

> *"`veloren-server-cli` declares nothing."*
> *"not one of them can be attributed to a build."*

**Both are false.** `server/src/lib.rs:1095` has always logged:

```
INFO veloren_server: Server version: 31b5928d [2026-08-13]
```

It appears in **63** of this session's server logs — including both arms of the run whose
void I attributed to a missing fingerprint:

| log | Server version |
|---|---|
| `server-cancel-plantappend-boot1.log` *(the VOID plant run)* | **31b5928d** |
| `server-cancel-ctrlfinal-boot1.log` *(the control)* | **9a832215** |

**They differ. The void was diagnosable from the log the whole time.** I reached for
`ls -la` on mtimes when the answer was already printed in the evidence I had collected.

This is the *read the content, not the label* failure in its purest form: I asserted an
absence without grepping for it, in logs I had generated myself.

## 2 · WHAT THE REAL GAP IS — narrower, and still real

`common/build.rs` derives the version from:

```
git log -n 1 --pretty=format:%h/%ct --abbrev=8
```

**HEAD only. There is no dirty-tree marker.** So the hash names *the commit HEAD sat at
when the crate was built*, and says nothing about uncommitted working-tree content.

That is exactly what misled me. I built persistence stages 2–3 while HEAD was still
`31b5928d` (stage 1) and committed them later as `9a832215`. Those binaries reported
`31b5928d` **while containing code that commit does not have**. The hash was *correct
about HEAD* and *misleading about content* — the most dangerous combination, because it
looks like provenance.

## 3 · THE REVISED BARS

### V1 · **A DIRTY BUILD SAYS SO**
- **PASS:** a binary built with uncommitted changes in the working tree reports a version
  carrying a **dirty** marker.
- **FAIL:** today — a dirty build is indistinguishable from a clean one at the same HEAD.

### V2 · **A CLEAN BUILD DOES NOT**
- **PASS:** committed tree ⇒ no marker. The matched control; without it a marker that is
  always on would pass V1 and mean nothing.

### V3 · **THE EXISTING HASH IS UNHARMED**
- `common::util::GIT_HASH` parses the version string with
  `u32::from_str_radix(split('/').nth(1))`. **Appending to the hash field would panic at
  startup.** The marker therefore travels as its own value, not glued onto the hash.
- **PASS:** `Server version:` still prints its 8-hex hash and the server still boots.

### PLANT
- Force the dirty probe to always report clean ⇒ **V1 red, V2 green** — isolating
  *detects dirt* from *emits a field*.

## 4 · COST, STATED BEFORE COMMITTING TO IT

`common/build.rs` is a **build script**: changing it rebuilds the whole workspace, not one
crate. That is a materially larger build than any row this session. It is worth it —
this defect silently corrupted provenance for an entire session's evidence — but the
build runs in the background, in its own call, per the rule the same void taught.

## 5 · WHAT I WILL **NOT** DO

1. **I will not quietly rewrite the prereg.** It was wrong in a specific, instructive way,
   and the correction is the point.
2. **I will not claim the fingerprint is new.** It exists; this row makes it *honest about
   dirt*, which is a much smaller and much better-founded claim.
3. **I will not append the marker to the hash field.** `GIT_HASH` would panic on a
   non-hex suffix — verified by reading the parse, not assumed.
