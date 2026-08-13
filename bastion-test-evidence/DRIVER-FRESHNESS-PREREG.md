# DRIVER-BINARY FRESHNESS (F3) — **PRE-REGISTRATION**

Written before any code change. Discharges **F3** from `WORLDGEN-PRESET-RESULTS.md`.

## 1 · THE FINDING

The `no_overflow` `bastion_playtest.exe` was built **2026-08-11 18:39**; targeted spawn
was committed **2026-08-12 18:13**. The old parser took the count and **silently
discarded the three coordinates**. Nine lattice origins all became the anchor position,
and the census read as *"all nine points are identical"* — **a perfectly consistent
lie**.

It was caught only because the driver happens to echo the position it sent, and I
happened to notice the `targeted=` field was **absent**. That is luck. The same binary
also drove the A4 plant/control pair (harmless there — both arms used identical
positions — but I did not know that at the time).

## 2 · WHAT ACTUALLY FAILED — not "the binary was old"

Staleness is the *cause*; the *defect* is that staleness was **silent**. Two properties
were missing:

1. **The parser accepted arguments it ignored.** `spawn 8 x y z` on the old binary
   parsed as `spawn 8`. A driver that had rejected the extra arguments would have failed
   loudly at line 1 instead of producing 24 minutes of confident, wrong evidence.
2. **No evidence log can be attributed to a binary.** Nothing in a driver log says which
   build produced it. Every log this session is, strictly, unattributed.

Fixing only (1) leaves every historical log unattributable. Fixing only (2) leaves the
next silent-discard to be caught by luck again. **Both, or the row is half done.**

## 3 · THE BARS

### D1 · **THE DRIVER DECLARES ITS BUILD AND ITS VERB TABLE**
- **PASS:** the driver's first log line names its **git commit** (compile-time), and the
  **verb table** it accepts.
- Derived, not asserted: the verb count is whatever the parser actually supports —
  currently **10** (`wait`, `anchor`, `spawn`, `designate`, `cancel`, `inspect_cell`,
  `list_designations`, `survey`, `note`, `cmd`). *I miscounted these as six once already
  this session; the table must come from the code, not from me.*

### D2 · **THE PARSER REFUSES ARGUMENTS IT WOULD IGNORE**
- **PASS:** `spawn 8 1.0 2.0` (three args — neither 1 nor 4) is rejected **by name and
  line number**, not silently truncated.
- **PASS:** `spawn 8 x y z` yields a **targeted** spawn; `spawn 8` yields an untargeted
  one. This is the exact capability whose silent absence voided the census.

### D3 · **THE DECLARATION CANNOT DRIFT FROM THE PARSER**
- A capability list written *beside* the parser is a second implementation — the F8
  defect — and would go stale exactly like the binary did.
- **PASS:** a test drives **every verb in the declared table** through the real parser
  and requires each to parse. A verb in the table that the parser rejects, or a parser
  verb missing from the table, fails the test.

### D4 · **LIVE**
- **PASS:** the fingerprint line appears in a real driver log from a real run.

### PLANTS
1. **Restore the silent-discard** (`spawn` ignores extra args) ⇒ **D2 red**.
2. **Remove one verb from the declared table** ⇒ **D3 red**, proving D3 compares against
   the parser rather than against itself.

## 4 · WHAT I WILL **NOT** DO

1. **I will not use file mtimes as the guard.** mtime is not identity: a touched file, a
   restored backup, or a copied binary all lie. The fingerprint is a **commit**, embedded
   at compile time.
2. **I will not write the verb table by hand beside the parser.** If the table and the
   parser can disagree, the guard has the defect it exists to prevent.
3. **I will not retro-attribute this session's logs.** They were produced by binaries
   that carried no fingerprint; they are unattributed and stay that way. The guard starts
   from here — claiming otherwise would be inventing provenance.
4. **I will not claim F3 closed on D1 alone.** A fingerprint that nothing checks is
   decoration; D2 is the half that would have caught the actual failure.
