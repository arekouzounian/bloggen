---
title: Comprehensive Markdown Test
author: BlogGen Test Suite
date: 2026-01-15
tags:
  - rust
  - markdown
  - content-addressable
---

# Main Title

This document tests **all** supported markdown node types in BlogGen v2.

## Basic Formatting

Regular text with **bold**, *italic*, ~~strikethrough~~, and `inline code`.

## Lists

### Unordered

- Item 1
- Item 2
  - Nested item
- Item 3

### Ordered

1. First
2. Second
3. Third

### Task List

- [x] Completed task
- [ ] Pending task

## Links and Images

Direct link: [Example](https://example.com "Title")

Reference link: [Ref Link][ref]

Direct image: ![Alt text](/path/to/image.jpg)

Reference image: ![Image][img-ref]

[ref]: https://example.com "Reference Title"
[img-ref]: /path/to/reference-image.jpg "Image Title"

## Code

Inline code: `let x = 42;`

Block code:

```rust
fn main() {
    println!("Hello, BlogGen!");
}
```

## Tables

| Header 1 | Header 2 | Header 3 |
|----------|:--------:|---------:|
| Left     | Center   | Right    |
| A        | B        | C        |

## Blockquote

> This is a blockquote.
> It can span multiple lines.
>
> And have multiple paragraphs.

## Math

Inline math: $E = mc^2$

Block math:

$$
\int_{a}^{b} f(x) dx = F(b) - F(a)
$$

## Footnotes

Here's a sentence with a footnote[^1].

And another with a different footnote[^note].

[^1]: This is the first footnote.
[^note]: This is a named footnote.

## Raw HTML

<div class="custom-class">
  <p>This is raw HTML content.</p>
</div>

## Horizontal Rule

---

## All Together!

That's everything we support!
