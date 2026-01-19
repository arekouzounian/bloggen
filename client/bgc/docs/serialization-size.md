# Content-Addressable Storage Serialization Analysis

## Executive Summary

The current JSON serialization of our content-addressable AST format produces **20-40x size bloat** compared to the input markdown. This document analyzes the root causes and proposes solutions that maintain the architectural benefits of content-addressable storage while dramatically reducing serialization overhead.

**Key Findings:**
- Hash arrays (`[u8; 32]` serialized as JSON) account for ~40-50% of total size
- JSON structural overhead adds another ~30-40%
- Switching to hex-encoded hashes reduces hash overhead by ~30%
- Binary formats (MessagePack, CBOR) can reduce total size by 60-70%

**Recommended Approach:** Hex-encoded hashes + MessagePack/CBOR for network transport, with backward-compatible migration path.

---

## Problem Statement

### Current Performance

Test case: Simple markdown document (124 bytes)
```markdown
# Test Document

This is a simple paragraph with some **bold** and *italic* text.

## Section 2

- Item 1
- Item 2
- Item 3
```

**Results:**
- Input size: 124 bytes
- JSON output size: 5,074 bytes
- **Size ratio: 40.9x**

Even simpler test (25 bytes: `# Hello\n\nThis is a test.\n`):
- JSON output: 1,066 bytes
- **Size ratio: 42.6x**

This bloat makes the format problematic for:
- Network transmission (even with compression, 10-20x bloat remains)
- Database storage (PostgreSQL jsonb benefits from compression but not enough)
- Client-side caching (FUSE driver will cache large payloads)

---

## Root Cause Analysis

### 1. Hash Representation: The Primary Culprit

**Current Implementation:**
```rust
pub type Blake3Hash = [u8; 32];

// In AstNode variants:
Root { children: Vec<Blake3Hash> }
```

**JSON Serialization:**
```json
{
  "type": "paragraph",
  "children": [
    [68,208,93,203,217,78,87,165,24,239,23,136,232,61,107,100,218,32,16,18,22,83,82,160,172,208,32,125,139,117,77,185]
  ]
}
```

**Size Breakdown for One Hash:**
- **Byte array in JSON**: ~95 characters average
  - 32 bytes, each serialized as 1-3 digit number
  - Average byte value: 127.5 → average 3 characters per byte
  - Plus 31 commas
  - Formula: 32 × 3 + 31 = **127 chars** (worst case: 96-127)

- **Hex string alternative**: 66 characters
  - 2 quotes + 64 hex characters
  - Formula: `"` + 64 hex + `"` = **66 chars**

**Savings per hash: 61 characters (~48% reduction in hash representation)**

### 2. JSON Structural Overhead

JSON adds significant overhead:
- Field names repeated for every node (`"type"`, `"children"`, `"value"`)
- Quotes around all strings
- Commas, brackets, braces for structure
- Whitespace in pretty-printing (though compact mode eliminates this)

**Example:**
```json
{"type":"text","value":"Hello"}
```
- Actual data: `text` (4 bytes) + `Hello` (5 bytes) = **9 bytes**
- JSON representation: **33 bytes**
- Overhead: **24 bytes (266%)**

### 3. Node Key Redundancy

Hash keys in the `nodes` map are **already hex-encoded** (good!):
```json
{
  "nodes": {
    "5acec791bfe54cbed64e1beb48073cab8c51c8ffc3054667c35d69344f18815a": {
      "type": "text",
      "value": "Hello"
    }
  }
}
```

But child references are byte arrays, creating an inconsistency and wasting space.

### 4. Deduplication Not Visible in Size

Content-addressable storage's main benefit is deduplication across documents. However:
- Within a single document, there's typically low redundancy
- A blog post rarely has identical paragraphs or headings
- Deduplication benefits appear at the **database level** (shared nodes across posts)
- Single-document serialization doesn't show this benefit

---

## Existing Design Goals & Constraints

From `design/cas-ast-spec.md`, our system must support:

### ✅ Must Preserve

1. **Content-addressable storage**: Nodes identified by hash of their content
2. **Deduplication**: Identical subtrees stored once (across all posts)
3. **Efficient updates**: Delta-based transmission (only changed nodes)
4. **Version history**: Track changes via root hash snapshots
5. **FUSE compatibility**: Random access to individual nodes
6. **Bidirectional conversion**: Markdown ↔ AST ↔ HTML

### ✅ Design Decisions

- **Blake3 hashing**: Fast, secure, 32-byte output (non-negotiable for security/performance)
- **Children as hashes**: Required for content-addressing (not inline content)
- **PostgreSQL with jsonb**: Queryable storage with GIN indexing
- **Delta updates**: Only transmit changed nodes, not full documents

### ❓ Open Questions (from spec)

The spec explicitly calls out serialization as an open question:

> **1. Serialization Format**
>
> **Options**:
> - **JSON** (jsonb in Postgres): Human-readable, queryable, ~2x larger
> - **Binary** (postcard/bincode): Compact, fast, opaque
> - **Hybrid**: JSON in DB (for queries), binary on wire
>
> **Recommendation**: Start with JSON for debugging, add binary optimization later.

**We are at this decision point now.** The spec anticipated 2x bloat; we're seeing 20-40x.

---

## Proposed Solutions

### Solution 1: Hex-Encoded Hashes (Quick Win)

**Change:**
```rust
// Instead of:
pub type Blake3Hash = [u8; 32];

// Use:
pub type Blake3Hash = String;  // Hex-encoded

// Or with newtype for type safety:
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Blake3Hash(#[serde(with = "hex_serde")] [u8; 32]);
```

**Impact:**
- Reduces hash representation from ~95 chars to 66 chars (**~30% savings on hashes**)
- Makes child refs consistent with node keys (both hex strings)
- Still JSON (human-readable, debuggable, jsonb compatible)
- **Estimated total size reduction: 15-20%** (since hashes are ~40% of total)

**Pros:**
- Easy to implement (custom Serialize/Deserialize for Blake3Hash)
- Maintains JSON format for debugging
- No breaking changes to database schema
- Still works with PostgreSQL jsonb + GIN indexes

**Cons:**
- Only addresses one source of bloat
- Still 25-30x size ratio (better than 40x, but not great)
- Doesn't solve JSON structural overhead

**Migration:**
- Transparent at serialization boundary
- Existing data in DB can be migrated with jsonb update query

---

### Solution 2: Binary Serialization (Major Win)

**Options:**

| Format | Size Reduction | Features | Rust Support |
|--------|---------------|----------|--------------|
| **MessagePack** | 60-70% | Schema-less, self-describing | `rmp-serde` (excellent) |
| **CBOR** | 55-65% | JSON-like, extensible | `ciborium` (good) |
| **Bincode** | 70-80% | Rust-native, not self-describing | `bincode` (excellent) |
| **Postcard** | 75-85% | No-std, minimal overhead | `postcard` (excellent) |

**Recommendation: MessagePack or CBOR**
- Self-describing (can evolve schema without versioning hell)
- Wide ecosystem support (not Rust-specific)
- JSON-compatible data model (easy migration)
- PostgreSQL has CBOR extensions available

**Impact:**
- **60-70% size reduction** from current JSON
- From 40x bloat → **6-12x bloat** (still bloated, but manageable)
- With compression (gzip): **3-5x bloat** (acceptable for network)

**Pros:**
- Dramatic size reduction
- Faster serialization/deserialization than JSON
- Still supports complex nested structures
- Works with existing AST design (no code changes except ser/de)

**Cons:**
- Not human-readable (harder debugging)
- PostgreSQL jsonb benefits lost (would need `bytea` column)
- Can't query AST structure in SQL (but do we need to?)

---

### Solution 3: Hybrid Approach (Best of Both Worlds)

**Architecture:**

```
┌─────────────────┐
│  Client (Rust)  │
│                 │
│  AST in memory  │
└────────┬────────┘
         │
         ├─────────────────┬──────────────────┐
         │                 │                  │
         │ Network         │ Database         │ Debug/Export
         │ (MessagePack)   │ (JSON/jsonb)     │ (JSON)
         ▼                 ▼                  ▼
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│   Server    │   │  Postgres   │   │  CLI Tool   │
│  (binary)   │   │  (jsonb)    │   │  (pretty)   │
└─────────────┘   └─────────────┘   └─────────────┘
```

**Implementation:**
- **Wire format**: MessagePack/CBOR (compact, fast)
- **Database format**: JSON (queryable, supports jsonb features)
- **Debug/CLI**: Pretty JSON (human-readable)

**Code:**
```rust
impl CasDocument {
    // Network serialization
    pub fn to_msgpack(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec(self)
    }

    // Database serialization (existing)
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
    }

    // Debug (existing)
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
    }
}
```

**Pros:**
- Compact network transmission (60-70% smaller)
- Queryable database storage (keep jsonb benefits)
- Human-readable debugging (CLI tools, logs)
- Best performance where it matters (network, API)

**Cons:**
- Two serialization formats to maintain
- Server must support both (but trivial with serde)
- Database still stores larger JSON (but storage is cheap, bandwidth isn't)

---

### Solution 4: Reference Compression (Advanced)

**Observation:** Within a document, hash references follow patterns.

**Technique:** Use a hash index table.

**Format:**
```json
{
  "root_hash": 0,  // Index into hash_table
  "hash_table": [
    "103243e5cc63e7d7510470c33fa33f1346fe4df1e1872a717d099eb05ab1caea",
    "4033919a8dda26345fa0c5a04f55f619cea2eed53b77727b6e059c31bf5af831",
    ...
  ],
  "nodes": {
    "0": { "type": "root", "children": [1, 2] },
    "1": { "type": "heading", "level": 1, "children": [3] },
    ...
  }
}
```

**Impact:**
- Hash refs: 66 chars → 1-2 chars (**~97% savings on child refs**)
- Hash keys: 64 chars → 1-2 chars (**~98% savings on keys**)
- **Estimated total reduction: 50-60%** combined with hex encoding

**Pros:**
- JSON stays human-readable (indices are clear)
- Massive reduction in hash repetition
- Easy to decode (look up index in table)

**Cons:**
- Breaks content-addressing at serialization level (hashes not self-contained)
- Complicates partial deserialization
- Hash table must be included in every document
- Less suitable for streaming/partial access

---

## Comparison Matrix

| Solution | Size Reduction | Maintains JSON | Queryable DB | Implementation Effort | Breaking Change |
|----------|---------------|----------------|--------------|----------------------|-----------------|
| **Current** | 0% (baseline) | ✅ | ✅ | N/A | N/A |
| **Hex Hashes** | 15-20% | ✅ | ✅ | Low (1-2 hours) | No |
| **Binary (MsgPack)** | 60-70% | ❌ | ❌ | Low (2-4 hours) | Yes (wire format) |
| **Hybrid** | 60-70% wire, 0% DB | ✅ (DB) | ✅ | Medium (4-8 hours) | No (additive) |
| **Reference Compression** | 50-60% | ✅ | Partial | High (1-2 days) | Yes (format) |

---

## Impact on Design Goals

### ✅ Preserved by All Solutions

- **Content-addressable storage**: Hashing unchanged (still Blake3)
- **Deduplication**: Happens at storage layer, not serialization
- **Version history**: Root hash tracking unaffected
- **FUSE compatibility**: All formats can be deserialized to AST
- **Bidirectional conversion**: Markdown ↔ AST still works

### ⚠️ Trade-offs

#### Binary Formats (MessagePack/CBOR)

**Lost:**
- ❌ Human-readable wire format (debugging harder)
- ❌ PostgreSQL jsonb benefits (GIN indexes, JSON operators)

**Gained:**
- ✅ 60-70% size reduction on network
- ✅ Faster serialization/deserialization
- ✅ Lower bandwidth costs

**Mitigation:**
- Keep JSON in database (hybrid approach)
- Use JSON for debug CLI output
- Binary only for client↔server communication

#### Hex-Encoded Hashes

**Lost:**
- Nothing significant

**Gained:**
- ✅ 15-20% size reduction
- ✅ Consistency (hashes always hex strings)

**Mitigation:**
- None needed (strictly better)

---

## Recommendations

### Phase 1: Quick Win (Implement Now)

**Action:** Switch to hex-encoded hashes in child references

**Implementation:**
```rust
// In src/ast.rs, add custom serialization:
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Blake3Hash(
    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex")]
    [u8; 32]
);

fn as_hex<S>(hash: &[u8; 32], ser: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    ser.serialize_str(&hex::encode(hash))
}

fn from_hex<'de, D>(de: D) -> Result<[u8; 32], D::Error>
where D: Deserializer<'de> {
    let s = String::deserialize(de)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}
```

**Expected Result:**
- 15-20% size reduction
- Maintains JSON format
- No API breaking changes

**Time Estimate:** 2-4 hours

---

### Phase 2: Optimize Network (Next Iteration)

**Action:** Add MessagePack/CBOR for client↔server communication

**Implementation:**
```rust
// Add to Cargo.toml:
// rmp-serde = "1.1"

impl CasDocument {
    pub fn to_msgpack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    pub fn from_msgpack(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}
```

**Server API:**
```
POST /posts/:slug/update
Content-Type: application/msgpack
Accept: application/msgpack

(Binary payload)
```

**Expected Result:**
- 60-70% size reduction on network
- Database still uses JSON (queryable)
- Debug output still pretty JSON

**Time Estimate:** 4-8 hours

---

### Phase 3: Consider Advanced Optimizations (If Still Needed)

After Phase 1 & 2, reassess:
- If network size is acceptable (with compression): DONE
- If database size is a problem: Consider binary DB storage or reference compression
- If performance is an issue: Profile and optimize hot paths

---

## Conclusion

The 20-40x size bloat is primarily due to:
1. **Hash representation** (40-50% of problem): Byte arrays → ~95 chars in JSON
2. **JSON overhead** (30-40% of problem): Structural syntax, field names, quotes
3. **Lack of compression** (20-30% of problem): No gzip on wire by default

**Recommended Path Forward:**

1. ✅ **Immediate (Phase 1)**: Hex-encode hashes → 15-20% reduction
2. ✅ **Next (Phase 2)**: MessagePack for network → 60-70% reduction on wire
3. ❓ **Future (Phase 3)**: Evaluate if further optimization needed

This approach:
- Preserves all architectural benefits (content-addressing, deduplication, FUSE)
- Maintains JSON in database (queryable, debuggable)
- Dramatically reduces network payload (the actual bottleneck)
- Allows iterative improvement without breaking changes

The original design spec's "Start with JSON for debugging, add binary optimization later" was sound advice. We've debugged, now it's time to optimize.
