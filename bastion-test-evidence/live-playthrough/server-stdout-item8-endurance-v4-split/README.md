# Split log: `server-stdout-item8-endurance-v4.log`

**Why split, not gzipped or filtered:** the raw v4 log is 278,753,195
bytes (~265.8 MB) — GitHub hard-rejects any single file over 100 MB
(`GH001`), a limit the pre-ruled "commit raw, no gzip, no filter"
decision never anticipated because no prior run's log came close to it.
Splitting is the only accommodation that satisfies GitHub's hard limit
**without violating either half of the pre-ruling**: no compression, no
content filtering — every byte is here, unmodified, just divided.

**Reassemble exactly:**

    cat part-000 part-001 part-002 > server-stdout-item8-endurance-v4.log

**Verified lossless before committing** — `md5sum` of the reassembled
stream matches the original file's `md5sum` exactly
(`9ff63ed3c8914873dd1f53a82959b445`, both sides).

Split via `split -b 90M -d -a3`, three parts (94,371,840 / 94,371,840 /
90,009,515 bytes), each comfortably under the 100 MB limit.
