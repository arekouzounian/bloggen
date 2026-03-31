# BlogGen v2 Frontend

A Next.js 14 frontend for the BlogGen v2 blogging platform.

## Quick Start

```bash
# Install dependencies
npm install

# Set up environment (copy and edit as needed)
cp .env.local.example .env.local

# Run development server (port 3001 to avoid conflicting with backend on 3000)
npm run dev
```

The frontend will be available at http://localhost:3001.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `NEXT_PUBLIC_API_URL` | `http://localhost:3000` | BlogGen backend server URL |

## Pages

- `/` — Home page: lists all published posts
- `/posts/[slug]` — Post page: displays a post's content

## Design Discussion: Rendering Approaches

The BlogGen backend can serve post content in three different ways. Each has trade-offs.

### Option A: Return the raw CAS AST (`GET /posts/:slug/ast/json`)

The server returns the content-addressed AST as a flat JSON map of `{ hash → node }` plus a root hash. The client traverses the tree by following hash references.

**Example response:**
```json
{
  "root_hash": "abc123...",
  "nodes": {
    "abc123...": { "type": "root", "children": ["def456..."] },
    "def456...": { "type": "heading", "level": 1, "children": ["ghi789..."] },
    "ghi789...": { "type": "text", "value": "Hello World" }
  }
}
```

**Pros:**
- Maximum flexibility on the client — can render however you like
- Deduplication: repeated subtrees are fetched/stored once
- Enables future delta updates (only transfer changed nodes)
- Client can cache individual nodes across posts

**Cons:**
- Complex client-side traversal logic
- Larger initial payload if the tree is deep
- Requires a hash-aware traversal loop on the frontend

**Best for:** Power users who want full control; future delta-sync features; long-term architecture.

---

### Option B: Return pre-rendered HTML (`GET /posts/:slug/html`)

The server walks the AST and generates a full HTML string, returned as `text/html`.

**Example response:**
```html
<h1>Hello World</h1><p>This is a paragraph with <strong>bold</strong> text.</p>
```

**Pros:**
- Trivially simple frontend — just `dangerouslySetInnerHTML` or inject into an iframe
- Server does all the work; no client-side logic needed
- Works with any HTML-capable frontend framework or even plain JavaScript

**Cons:**
- Tight coupling: visual changes require server redeployment
- No easy way to apply per-user or per-theme styling beyond global CSS
- `dangerouslySetInnerHTML` requires careful XSS sanitization (the server escapes HTML, but you must trust the server)
- Less interactive — hard to add React-powered components inside server-rendered HTML

**Best for:** Quick proof-of-concepts; blogs with no dynamic components; environments where the server and frontend are tightly coupled.

---

### Option C: Intermediate representation (HTML-oriented JSON)

Add a new server endpoint that walks the AST and produces a simplified, tag-centric JSON tree (similar to React's virtual DOM or VDOM):

```json
{
  "tag": "root",
  "children": [
    {
      "tag": "h1",
      "children": [{ "tag": "text", "value": "Hello World" }]
    },
    {
      "tag": "p",
      "children": [
        { "tag": "text", "value": "This is " },
        { "tag": "strong", "children": [{ "tag": "text", "value": "bold" }] },
        { "tag": "text", "value": " text." }
      ]
    }
  ]
}
```

**Pros:**
- Easier for the frontend to render than the CAS AST (no hash traversal, already tree-shaped)
- More flexible than raw HTML strings (can swap out rendering logic without a server change)
- Natural mapping to React component trees
- Can include metadata like CSS class hints, IDs, ARIA attributes

**Cons:**
- Another serialization format to maintain
- Still requires client-side rendering logic (though simpler than the raw AST)
- Loses CAS deduplication benefits

**Best for:** A good middle ground between A and B. Recommended if you want a React-rendered frontend that doesn't need delta sync.

---

### Recommendation

For the **current proof of concept**, **Option A (raw AST)** is used so we can validate the data pipeline end-to-end with zero additional server work.

For **production**, **Option C (intermediate representation)** is recommended:
- It decouples presentation from the server's internal CAS representation
- It provides a stable, frontend-friendly API contract
- It's simpler to render in React than the hash-linked CAS tree

**Option B (HTML)** is a viable shortcut if the team wants to ship quickly and doesn't need rich client-side interactivity.

A concrete migration path:
1. ✅ Today: Render raw AST (proof of concept — this is what we've built)
2. 🔜 Next: Add `GET /posts/:slug/vtree` endpoint returning Option C IR, render in React
3. 🔜 Future: Add delta sync using Option A CAS hashes for incremental updates
