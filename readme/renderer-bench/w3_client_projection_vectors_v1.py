# W3 client-projection vectors — INDEPENDENT producer (derives from the
# W0 script lineage `renderer_r0d_vectors_v1.py`, NOT from the Rust
# encoder: the production encoder never blesses its own vectors).
# Emits w3-client-projection-vectors-v1.json consumed by
# common/tests/renderer_bench_vectors.rs.
from __future__ import annotations
import hashlib, struct, json, sys

H = lambda b: hashlib.sha256(b).digest()

def u8(v): return struct.pack('<B', v)
def u16(v): return struct.pack('<H', v)
def u32(v): return struct.pack('<I', v)
def i32(v): return struct.pack('<i', v)
def lp(b): return u32(len(b)) + b

TYPE_FIXED_I32 = 16
DOMAIN_CLIENT_PROJECTION = 5
OWNER_STABLE_ENTITY = 3
LEAF_ID = 0x05000001

schema = H(b'BASTION:R0D:ORACLE-SCHEMA:VECTOR:V1\0')

def leaf(semantic_id, mm):
    owner = u32(semantic_id)
    payload = i32(mm[0]) + i32(mm[1]) + i32(mm[2])
    pre = (b'bastion.renderer.leaf.v1\0' + schema
           + u16(DOMAIN_CLIENT_PROJECTION) + u32(LEAF_ID)
           + u16(TYPE_FIXED_I32) + u8(OWNER_STABLE_ENTITY)
           + lp(owner) + lp(payload))
    return H(pre)

def owner_root(semantic_id, mm):
    owner = u32(semantic_id)
    pre = (b'bastion.renderer.owner.v1\0' + schema
           + u8(OWNER_STABLE_ENTITY) + lp(owner)
           + u32(1) + u32(LEAF_ID) + leaf(semantic_id, mm))
    return H(pre)

def composite(semantic_id):
    return u8(OWNER_STABLE_ENTITY) + lp(u32(semantic_id))

def domain_root(entries):
    # entries: [(semantic_id, mm)] — MUST already be sorted by semantic id.
    pre = (b'bastion.renderer.domain.v1\0' + schema
           + u16(DOMAIN_CLIENT_PROJECTION) + u32(len(entries)))
    for semantic_id, mm in entries:
        pre += lp(composite(semantic_id)) + owner_root(semantic_id, mm)
    return H(pre)

single = (42, (1500, -2500, 0))
triple = [(1, (0, 0, 0)), (2, (-1, 250000, -999)), (7, (2147483647, -2147483648, 1))]

out = {
    "schema": "w3-client-projection-vectors-v1",
    "tags": {
        "domain": DOMAIN_CLIENT_PROJECTION,
        "leaf_id": LEAF_ID,
        "wire_type": TYPE_FIXED_I32,
        "owner_kind": OWNER_STABLE_ENTITY,
    },
    "single": {
        "semantic_id": single[0],
        "mm": list(single[1]),
        "leaf": leaf(*single).hex(),
        "owner_root": owner_root(*single).hex(),
        "composite": composite(single[0]).hex(),
        "domain_root": domain_root([single]).hex(),
    },
    "triple": {
        "entries": [{"semantic_id": s, "mm": list(m)} for s, m in triple],
        "domain_root": domain_root(triple).hex(),
    },
    "empty_domain_root": domain_root([]).hex(),
}
path = sys.argv[1] if len(sys.argv) > 1 else "w3-client-projection-vectors-v1.json"
with open(path, "w") as f:
    json.dump(out, f, indent=1)
print("wrote", path)
