# Example: Converting to Compile-Time Checked Queries

This shows how the current unchecked queries could be converted to checked queries after setting up SQLx offline mode.

## Current (Unchecked) vs Checked

### NodeStore::insert_node

**Current:**
```rust
sqlx::query(
    r#"
    INSERT INTO ast_nodes (hash, node_data, ref_count)
    VALUES ($1, $2, 0)
    ON CONFLICT (hash) DO NOTHING
    "#,
)
.bind(hash.as_bytes() as &[u8])
.bind(node_json)
.execute(&self.pool)
.await?;
```

**With compile-time checking:**
```rust
sqlx::query!(
    r#"
    INSERT INTO ast_nodes (hash, node_data, ref_count)
    VALUES ($1, $2, 0)
    ON CONFLICT (hash) DO NOTHING
    "#,
    hash.as_bytes() as &[u8],
    node_json
)
.execute(&self.pool)
.await?;
```

**Benefits:**
- Validates column names at compile time
- Validates types (BYTEA, JSONB, INTEGER)
- Catches typos in SQL

---

### NodeStore::get_node

**Current:**
```rust
let row: Option<(sqlx::types::JsonValue,)> = sqlx::query_as(
    r#"
    SELECT node_data
    FROM ast_nodes
    WHERE hash = $1
    "#,
)
.bind(hash.as_bytes() as &[u8])
.fetch_optional(&self.pool)
.await?;

match row {
    Some((node_data,)) => {
        let node: AstNode = serde_json::from_value(node_data)?;
        Ok(Some(node))
    }
    None => Ok(None),
}
```

**With compile-time checking:**
```rust
let row = sqlx::query!(
    r#"
    SELECT node_data
    FROM ast_nodes
    WHERE hash = $1
    "#,
    hash.as_bytes() as &[u8]
)
.fetch_optional(&self.pool)
.await?;

match row {
    Some(row) => {
        let node: AstNode = serde_json::from_value(row.node_data)?;
        Ok(Some(node))
    }
    None => Ok(None),
}
```

**Benefits:**
- `row.node_data` is a compile-time known field
- Type is automatically inferred as `JsonValue`
- IDE autocomplete works

---

### RefCountOps::increment_refs

**Current:**
```rust
sqlx::query(
    r#"
    UPDATE ast_nodes
    SET ref_count = ref_count + 1
    WHERE hash = ANY($1)
    "#,
)
.bind(&hash_bytes)
.execute(&self.pool)
.await?;
```

**With compile-time checking:**
```rust
sqlx::query!(
    r#"
    UPDATE ast_nodes
    SET ref_count = ref_count + 1
    WHERE hash = ANY($1)
    "#,
    &hash_bytes
)
.execute(&self.pool)
.await?;
```

**Benefits:**
- Validates `ref_count` column exists
- Validates `ref_count` is numeric (can do addition)
- Catches typos like `ref_counts` or `refcount`

---

## Errors Caught at Compile Time

### Example 1: Typo in column name
```rust
sqlx::query!(
    "SELECT node_datas FROM ast_nodes"  // Typo: node_datas
)
```
**Error:**
```
error: no such column: node_datas
```

### Example 2: Type mismatch
```rust
let count: String = sqlx::query_scalar!(
    "SELECT COUNT(*) FROM ast_nodes"
)
.fetch_one(&pool)
.await?;
```
**Error:**
```
error: expected `String`, found `i64`
```

### Example 3: Missing table
```rust
sqlx::query!(
    "SELECT * FROM ast_nodez"  // Typo: ast_nodez
)
```
**Error:**
```
error: relation "ast_nodez" does not exist
```

---

## When to Use Each Approach

### Use Unchecked Queries (`sqlx::query()`) when:
- Setting up a new project (no database yet)
- Writing tests with mock implementations
- Need maximum build speed
- Don't have Docker/PostgreSQL available

### Use Checked Queries (`sqlx::query!()`) when:
- Database schema is stable
- Want maximum safety
- Have CI/CD set up with offline mode
- Team has standardized on local development databases

---

## Migration Strategy

1. **Set up database:** `make setup`
2. **Convert one module at a time** (e.g., start with `gc.rs`)
3. **Test thoroughly** after each conversion
4. **Commit `.sqlx/` directory** for offline builds
5. **Update CI to use** `SQLX_OFFLINE=true`

---

## Performance Impact

**Compile time:**
- Unchecked: Fast (no database connection needed)
- Checked (live DB): Slower (connects to DB during build)
- Checked (offline): Fast (uses cached metadata)

**Runtime:**
- **No difference** - both compile to the same code

**Recommendation:** Use offline mode for best of both worlds.
