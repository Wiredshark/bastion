# W4 visual-domain vectors — INDEPENDENT producer (same lineage as the W0
# scripts and the W3 producer; the Rust encoder never blesses itself).
# Emits w4-visual-domain-vectors-v1.json for renderer_bench_vectors.rs.
from __future__ import annotations
import hashlib, struct, json, sys

H = lambda b: hashlib.sha256(b).digest()
def u8(v): return struct.pack('<B', v)
def u16(v): return struct.pack('<H', v)
def u32(v): return struct.pack('<I', v)
def u64(v): return struct.pack('<Q', v)
def lp(b): return u32(len(b)) + b

TYPE_STRUCT = 15
DOMAIN_PASS_DRAW = 12
DOMAIN_VISUAL_STRUCTURE = 13
OWNER_PASS = 6
OWNER_FRAME = 2
PASS_DRAW_LEAF = 0x0C000001
VISUAL_STRUCTURE_LEAF = 0x0D000001

schema = H(b'BASTION:R0D:ORACLE-SCHEMA:VECTOR:V1\0')

def leaf(domain, leaf_id, owner_kind, owner_key, payload):
    pre = (b'bastion.renderer.leaf.v1\0' + schema + u16(domain) + u32(leaf_id)
           + u16(TYPE_STRUCT) + u8(owner_kind) + lp(owner_key) + lp(payload))
    return H(pre)

def owner_root(owner_kind, owner_key, leaf_id, lh):
    pre = (b'bastion.renderer.owner.v1\0' + schema + u8(owner_kind)
           + lp(owner_key) + u32(1) + u32(leaf_id) + lh)
    return H(pre)

def domain_root(domain, owner_kind, owner_key, oroot):
    composite = u8(owner_kind) + lp(owner_key)
    pre = (b'bastion.renderer.domain.v1\0' + schema + u16(domain)
           + u32(1) + lp(composite) + oroot)
    return H(pre)

# The registered stats fixture (W4-LAUNCH-PACKET payload orders).
stats = dict(pass_count=3, draw_count=2615, instances=516, geometry_units=987654321,
             terrain_chunks=1801, visible_terrain_chunks=516,
             shadow_terrain_chunks=1200, figure_draw_count=27)
frame_index = 7

pd_payload = (u32(stats['pass_count']) + u32(stats['draw_count'])
              + u32(stats['instances']) + u64(stats['geometry_units']))
pd_key = u32(0)
pd_leaf = leaf(DOMAIN_PASS_DRAW, PASS_DRAW_LEAF, OWNER_PASS, pd_key, pd_payload)
pd_owner = owner_root(OWNER_PASS, pd_key, PASS_DRAW_LEAF, pd_leaf)
pd_domain = domain_root(DOMAIN_PASS_DRAW, OWNER_PASS, pd_key, pd_owner)

vs_payload = (u32(stats['terrain_chunks']) + u32(stats['visible_terrain_chunks'])
              + u32(stats['shadow_terrain_chunks']) + u32(stats['figure_draw_count']))
vs_key = u32(frame_index)
vs_leaf = leaf(DOMAIN_VISUAL_STRUCTURE, VISUAL_STRUCTURE_LEAF, OWNER_FRAME, vs_key, vs_payload)
vs_owner = owner_root(OWNER_FRAME, vs_key, VISUAL_STRUCTURE_LEAF, vs_leaf)
vs_domain = domain_root(DOMAIN_VISUAL_STRUCTURE, OWNER_FRAME, vs_key, vs_owner)

out = {
    "schema": "w4-visual-domain-vectors-v1",
    "tags": {"pass_draw": {"domain": DOMAIN_PASS_DRAW, "leaf_id": PASS_DRAW_LEAF, "owner_kind": OWNER_PASS},
             "visual_structure": {"domain": DOMAIN_VISUAL_STRUCTURE, "leaf_id": VISUAL_STRUCTURE_LEAF, "owner_kind": OWNER_FRAME},
             "wire_type": TYPE_STRUCT},
    "stats": stats, "frame_index": frame_index,
    "pass_draw": {"leaf": pd_leaf.hex(), "owner_root": pd_owner.hex(), "domain_root": pd_domain.hex()},
    "visual_structure": {"leaf": vs_leaf.hex(), "owner_root": vs_owner.hex(), "domain_root": vs_domain.hex()},
}
path = sys.argv[1] if len(sys.argv) > 1 else "w4-visual-domain-vectors-v1.json"
with open(path, "w") as f:
    json.dump(out, f, indent=1)
print("wrote", path)
