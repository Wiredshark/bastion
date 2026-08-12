# Split log: `server-stdout-item8-endurance-v5.log`

**Why split, not gzipped or filtered:** the raw v5 log is 161,749,162
bytes (~161.7 MB) — GitHub hard-rejects any single file over 100 MB
(`GH001`), same limit v4's log hit. Splitting is the only accommodation
that satisfies GitHub's hard limit without violating the "commit raw, no
gzip, no filter" pre-ruling from v3/v4: no compression, no content
filtering — every byte is here, unmodified, just divided.

**Reassemble exactly:**

    cat part-000 part-001 > server-stdout-item8-endurance-v5.log

**Verified lossless before committing** — `md5sum` of the reassembled
stream matches the original file's `md5sum` exactly
(`f04a4c72ba1a8c9775263cb183eb7705`, both sides).

Split via `split -b 90M -d -a3`, two parts (94,371,840 / 67,377,322
bytes), both comfortably under the 100 MB limit.
