# DRIVER-BINARY FRESHNESS (F3) — **RESULTS & ROW DISPOSITION**

Scored against `DRIVER-FRESHNESS-PREREG.md` (`d84bf156fe`). Engine tip `f430ee8854`.

## THE SCORE — **4 PASS, 0 FAIL**

| bar | verdict | evidence |
|---|---|---|
| **D1** build + verb table declared | ✅ PASS | `driver build=bbef73f9 built_at=1786645194 verbs=…` — **10** verbs |
| **D2** arguments refused, not ignored | ✅ PASS | 4 args ⇒ targeted; 1 ⇒ untargeted; 3 ⇒ panics by line |
| **D3** declaration cannot drift | ✅ PASS | parser **gates on** the table; every declared verb driven through it |
| **D4** live | ✅ PASS | fingerprint is line 2 of a real driver log |

| plant | required red | observed |
|---|---|---|
| silent discard restored (the 2026-08-11 behaviour) | D2 red | **only** the arity test fails — the other three green |
| `spawn` removed from the table | D3 red | the spawn tests fail — the table is load-bearing |

Restored: **4/4**, live fingerprint intact.

**`build=bbef73f9` matches engine tip `bbef73f9e5`** — the fingerprint is not merely
present, it is *correct*.

## ★ WRITING THE PLANT EXPOSED A WEAKNESS IN MY OWN DESIGN

My first cut declared `SCRIPT_VERBS` beside the parser and tested that every declared
verb parses. Then I went to run the registered plant — *remove a verb from the table* —
and realised **it would not have reddened anything**: the test would simply have checked
fewer verbs. The table could be under-declared silently, in the one direction that
matters, which is precisely the failure mode this row exists to prevent.

So the parser now **gates on the table**: an undeclared verb is rejected. The declaration
became the contract instead of a comment about it, and the plant now works — removing
`spawn` reddens the spawn tests.

**The plant found a hole in the guard before the guard ever shipped.** That is the second
time this session a registered plant did more than confirm a bar (the first was S1's
delegation plant, which needed the live tier).

## ⚠ THE FINGERPRINT'S LIMIT, NAMED

`GIT_HASH` derives from `VELOREN_GIT_VERSION`, and `common::util` lets a **runtime env
var override it**. Unset — as in every run here — it reports the build. Set, it would
lie. It is the strongest identity this crate exposes, and the caveat is carried in the
code beside the emit, not just in a commit message.

My prereg said "embedded at compile time". That is **not exactly true** of the value I
used, and I am correcting it here rather than letting the stronger claim stand.

## WHAT I DECLINE TO CLAIM

- **Not** that this session's earlier logs are attributable. They were produced by
  binaries carrying no fingerprint. **They are unattributed and stay that way** —
  retro-attributing them would be inventing provenance, which is worse than the gap.
- **Not** that mtime was ever a candidate. A touched file, a restored backup, or a copied
  binary all lie; identity is a commit.

## SESSION QUEUE STATE — all six rows closed

1. ✅ Founding preset on real worldgen — PASS (`f51213cc4c`)
2. ✅ Arena trees / F8-C1 — CLOSED (`793df9401a`)
3. ✅ S1 sentinel scored-bar — PASS (`dcc0b950e9`)
4. ✅ Water gate / F1 — PASS (`95a597ec5a`)
5. ✅ Relief-emit blind spot / F2 — PASS (`5801770cb5`)
6. ✅ **Driver freshness / F3 — PASS**, this document

**Every finding this session opened has now been closed by a later row in the same
session.** Next: the roadmap's open items — tick-driven loading spec, save/load
colony-state persistence, and §8 N2's widget tier (still the one acceptance tier no bar
has ever run at).
