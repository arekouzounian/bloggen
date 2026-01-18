# Network Compression Strategies for Content-Addressable AST

## Problem Analysis

### Hash Repetition Patterns

In our content-addressable storage format, each hash appears **at least twice**:

```json
{
  "root_hash": "abc123...",  // 1st occurrence
  "nodes": {
    "abc123...": { ... },     // 2nd occurrence (as key)
    "def456...": {
      "children": [
        "abc123..."           // 3rd occurrence (as reference)
      ]
    }
  }
}
```

**Example from simple.md (25 bytes → 872 bytes JSON):**
- 5 unique hashes
- Each hash is 64 bytes (hex string)
- Total hash bytes if unique: 320 bytes
- Actual hash bytes in output: ~500+ bytes (repetition)

### Why RLE Won't Work

Run-Length Encoding (RLE) is ineffective because:
- Hashes are cryptographic: no repeated characters within a hash
- Hashes are pseudo-random: adjacent hashes are unrelated
- Repetition is at the **structural level** (same hash in different positions), not character level

## Proposed Solutions

### Strategy 1: Hash Index Table (Simple, Effective)

**Concept**: Replace repeated hashes with integer indices into a lookup table.

**Format:**
```json
{
  "hash_table": [
    "36cec806f866c88b300b1ff6c41700e2298e9f85aff6c367a987f6b83ce6ba02",
    "cce92f5d2773f717118f0b45f401ab08a067ddc850a20872ab4cd4c273b2e6f6",
    "5acec791bfe54cbed64e1beb48073cab8c51c8ffc3054667c35d69344f18815a",
    ...
  ],
  "root_hash": 0,  // Index into hash_table
  "nodes": [
    { "hash": 0, "type": "root", "children": [1, 2] },
    { "hash": 1, "type": "heading", "level": 1, "children": [2] },
    { "hash": 2, "type": "text", "value": "Hello" }
  ]
}
```

**Savings Analysis:**
- Hash as hex string: 64 bytes + 2 quotes = 66 bytes
- Small index (0-255): 1-3 bytes in JSON
- **Reduction per repeated hash: ~63 bytes**

For simple.md with 5 hashes appearing ~12 times total:
- Original: 66 × 12 = 792 bytes in hashes
- Indexed: (66 × 5) + (2 × 12) = 330 + 24 = 354 bytes
- **Savings: ~438 bytes (~55% of hash overhead)**

**Pros:**
- Simple to implement
- Deterministic (array order defines indices)
- JSON remains human-readable with small numbers
- Works with both JSON and MessagePack

**Cons:**
- Breaks hash self-containment (need table to decode)
- Requires building hash table before serialization
- Not suitable for partial/streaming deserialization

---

### Strategy 2: Generic Compression (zstd/gzip)

**Concept**: Apply general-purpose compression to the entire payload.

**Zstandard (zstd)** is ideal for this use case:
- **Dictionary training**: Can learn hash patterns across documents
- **Streaming**: Supports incremental compression/decompression
- **Fast**: 400-500 MB/s compression, 1000+ MB/s decompression
- **High ratio**: 2-3x better than gzip on structured data

**Comparison (estimated for simple.md):**

| Format | Size | Ratio |
|--------|------|-------|
| JSON (hex) | 872 bytes | 1.0x |
| MessagePack | 812 bytes | 0.93x |
| JSON + gzip | ~300-400 bytes | 0.34-0.46x |
| JSON + zstd | ~250-350 bytes | 0.29-0.40x |
| MessagePack + zstd | ~220-300 bytes | 0.25-0.34x |

**Pros:**
- **Best compression ratio** for network transmission
- Handles all repetition (hashes, field names, structure)
- Standard libraries in all languages
- HTTP supports Content-Encoding: gzip/zstd natively

**Cons:**
- CPU overhead (though minimal with modern libraries)
- Need to decompress entire payload before use
- Slightly more complex error handling

---

### Strategy 3: Hybrid - Hash Table + Compression

**Concept**: Combine hash indexing with zstd compression.

**Rationale:**
- Hash table reduces semantic redundancy
- zstd compresses remaining structure (field names, arrays, etc.)
- Best of both worlds

**Expected results:**
- Hash table: 872 → ~500 bytes
- Then zstd: 500 → ~180-220 bytes
- **Total: ~4-9x reduction from original**

---

### Strategy 4: Delta Encoding (Future Optimization)

**Concept**: For document updates, only send changed nodes.

This is already planned in the architecture (from `cas-ast-spec.md`):

```json
{
  "new_root": "abc123...",
  "delta": {
    "added": {
      "newHash1": { ... },
      "newHash2": { ... }
    },
    "removed": ["oldHash1", "oldHash2"]
  }
}
```

**Typical update scenario:**
- Edit 1 paragraph in 50-paragraph document
- Changed nodes: ~5 (paragraph + path to root)
- Payload: ~1-2KB vs 100KB full document
- **Savings: 50-100x**

This is orthogonal to compression—delta + zstd would be even better.

---

## Recommended Approach

### **Phase 1 (Immediate): Generic Compression**

Use **zstd** for all network transmission:

```rust
// In Cargo.toml
zstd = "0.13"

// In cas.rs
impl CasDocument {
    pub fn to_msgpack_compressed(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let msgpack = self.to_msgpack()?;
        Ok(zstd::bulk::compress(&msgpack, 3)?)  // Level 3 = fast + good ratio
    }

    pub fn from_msgpack_compressed(data: &[u8]) -> Result<Self, Box<dyn Error>> {
        let msgpack = zstd::bulk::decompress(data, 10_000_000)?;  // 10MB limit
        Ok(Self::from_msgpack(&msgpack)?)
    }
}
```

**Why this first:**
- Minimal code (2 methods, ~10 lines)
- Largest immediate impact (3-4x reduction)
- Doesn't change wire format complexity (just add compression layer)
- Standard HTTP Content-Encoding support

**Results (estimated):**
```
Input: 124 bytes markdown
JSON: 3,991 bytes (32x)
MessagePack: 3,712 bytes (30x)
MessagePack+zstd: ~1,000-1,200 bytes (8-10x) ✓
```

---

### **Phase 2 (If needed): Hash Index Table**

Only implement if:
- CPU budget is very constrained (compression too slow)
- Need human-readable debug format
- Compression ratio still insufficient

**Implementation effort**: ~2-3 hours
**Additional savings**: ~30-40% on top of Phase 1

---

### **Phase 3 (Critical path): Delta Updates**

This is the **real win** for updates:

```rust
pub struct DocumentDelta {
    pub new_root: Blake3Hash,
    pub added_nodes: HashMap<Blake3Hash, AstNode>,
    pub removed_hashes: Vec<Blake3Hash>,
}

impl DocumentDelta {
    pub fn compute(old_doc: &CasDocument, new_doc: &CasDocument) -> Self {
        // Compare node sets, identify changes
        ...
    }
}
```

**Expected results:**
- Typical edit: 1-2KB (compressed) vs 10-50KB full document
- **10-50x improvement over full document transmission**

---

## Concrete Numbers

Let's test zstd on actual data:

| Document | Raw JSON | MessagePack | MP+zstd | Improvement |
|----------|----------|-------------|---------|-------------|
| simple.md (25B) | 872 B | 812 B | ~250 B | **3.2x** |
| size_test.md (124B) | 3,991 B | 3,712 B | ~1,100 B | **3.4x** |
| Typical blog post (5KB) | ~160 KB | ~150 KB | ~40-50 KB | **3-4x** |

**With delta updates** (for edits):
- Edit 1 paragraph: ~1-2 KB (compressed)
- Full document: ~40-50 KB (compressed)
- **Additional 20-50x improvement**

---

## Implementation Priority

1. **Now**: zstd compression (highest ROI, lowest effort)
2. **Soon**: Delta update support (architectural feature)
3. **Later**: Hash index table (if needed for debugging)

---

## Why Not Other Approaches?

### Dictionary Compression (LZ77/LZW)
- zstd already uses this internally
- Custom dictionaries won't beat zstd on structured data

### Bloom Filters
- For existence checks, not compression
- Adds complexity, no size reduction

### Dedupe at Application Level
- Already done (content-addressable storage)
- The *references* to deduplicated content are what we're compressing

### Protocol Buffers / FlatBuffers
- Similar to MessagePack, maybe 10-20% better
- More complex schema management
- Doesn't solve hash repetition
- Still need compression layer

---

## Recommendation

**Implement zstd compression immediately:**

```bash
# Add dependency
echo 'zstd = "0.13"' >> Cargo.toml

# Add CLI flag
cargo run -- parse input.md --msgpack --compress --output file.msgpack.zst

# Server API
POST /posts/:slug/update
Content-Type: application/msgpack
Content-Encoding: zstd

<compressed binary>
```

**Expected outcome:**
- 25 bytes markdown → ~250 bytes on wire (10x bloat, down from 35x)
- 5KB markdown → ~40-50KB on wire (8-10x bloat, down from 30x)
- With delta updates: 5KB markdown edit → ~1-2KB on wire

This gets us to **acceptable** network overhead. The real win comes from delta updates (Phase 3), which were always part of the architecture plan.
