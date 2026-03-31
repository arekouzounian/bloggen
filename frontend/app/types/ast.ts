/**
 * TypeScript types mirroring the bgc-ast Rust crate's AstNode enum.
 * These match the JSON serialization produced by the server's JSON endpoints.
 */

/** A 64-character hex string representing a Blake3 hash */
export type Blake3Hash = string;

// ── Leaf nodes (no children) ─────────────────────────────────────────────────

export interface TextNode {
  type: "text";
  value: string;
}

export interface CodeBlockNode {
  type: "code_block";
  lang: string | null;
  value: string;
}

export interface InlineCodeNode {
  type: "inline_code";
  value: string;
}

export interface ImageNode {
  type: "image";
  url: string;
  alt: string;
  title: string | null;
}

export interface BreakNode {
  type: "break";
}

export interface ThematicBreakNode {
  type: "thematic_break";
}

export interface HtmlNode {
  type: "html";
  value: string;
}

export interface DefinitionNode {
  type: "definition";
  identifier: string;
  label: string | null;
  url: string;
  title: string | null;
}

export interface ImageReferenceNode {
  type: "image_reference";
  reference_kind: ReferenceKind;
  identifier: string;
  label: string | null;
  alt: string;
}

export interface YamlNode {
  type: "yaml";
  value: string;
}

export interface TomlNode {
  type: "toml";
  value: string;
}

export interface FootnoteReferenceNode {
  type: "footnote_reference";
  identifier: string;
  label: string | null;
}

export interface MathNode {
  type: "math";
  value: string;
}

export interface InlineMathNode {
  type: "inline_math";
  value: string;
}

export interface MdxjsEsmNode {
  type: "mdxjs_esm";
  value: string;
}

export interface MdxFlowExpressionNode {
  type: "mdx_flow_expression";
  value: string;
}

export interface MdxTextExpressionNode {
  type: "mdx_text_expression";
  value: string;
}

// ── Parent nodes (have children stored as hash references) ───────────────────

export interface RootNode {
  type: "root";
  children: Blake3Hash[];
}

export interface HeadingNode {
  type: "heading";
  level: number;
  children: Blake3Hash[];
}

export interface ParagraphNode {
  type: "paragraph";
  children: Blake3Hash[];
}

export interface ListNode {
  type: "list";
  ordered: boolean;
  start: number | null;
  children: Blake3Hash[];
}

export interface ListItemNode {
  type: "list_item";
  checked: boolean | null;
  children: Blake3Hash[];
}

export interface BlockquoteNode {
  type: "blockquote";
  children: Blake3Hash[];
}

export interface StrongNode {
  type: "strong";
  children: Blake3Hash[];
}

export interface EmphasisNode {
  type: "emphasis";
  children: Blake3Hash[];
}

export interface DeleteNode {
  type: "delete";
  children: Blake3Hash[];
}

export interface LinkNode {
  type: "link";
  url: string;
  title: string | null;
  children: Blake3Hash[];
}

export interface TableNode {
  type: "table";
  children: Blake3Hash[];
}

export interface TableRowNode {
  type: "table_row";
  children: Blake3Hash[];
}

export interface TableCellNode {
  type: "table_cell";
  children: Blake3Hash[];
}

export interface LinkReferenceNode {
  type: "link_reference";
  reference_kind: ReferenceKind;
  identifier: string;
  label: string | null;
  children: Blake3Hash[];
}

export interface FootnoteDefinitionNode {
  type: "footnote_definition";
  identifier: string;
  label: string | null;
  children: Blake3Hash[];
}

export interface MdxJsxFlowElementNode {
  type: "mdx_jsx_flow_element";
  name: string | null;
  children: Blake3Hash[];
}

export interface MdxJsxTextElementNode {
  type: "mdx_jsx_text_element";
  name: string | null;
  children: Blake3Hash[];
}

// ── Union type ────────────────────────────────────────────────────────────────

export type AstNode =
  | RootNode
  | HeadingNode
  | ParagraphNode
  | ListNode
  | ListItemNode
  | BlockquoteNode
  | StrongNode
  | EmphasisNode
  | DeleteNode
  | TextNode
  | CodeBlockNode
  | InlineCodeNode
  | LinkNode
  | ImageNode
  | BreakNode
  | ThematicBreakNode
  | TableNode
  | TableRowNode
  | TableCellNode
  | HtmlNode
  | DefinitionNode
  | LinkReferenceNode
  | ImageReferenceNode
  | YamlNode
  | TomlNode
  | FootnoteDefinitionNode
  | FootnoteReferenceNode
  | MathNode
  | InlineMathNode
  | MdxjsEsmNode
  | MdxFlowExpressionNode
  | MdxTextExpressionNode
  | MdxJsxFlowElementNode
  | MdxJsxTextElementNode;

export type ReferenceKind = "full" | "collapsed" | "shortcut";

// ── API response types ────────────────────────────────────────────────────────

/** Response from GET /posts/:slug/ast/json */
export interface CasDocumentResponse {
  root_hash: Blake3Hash;
  /** Map from hex hash to AstNode */
  nodes: Record<Blake3Hash, AstNode>;
}

/** A post summary from GET /posts/json */
export interface PostSummary {
  slug: string;
  title: string | null;
  created_at: string;
  updated_at: string;
  published: boolean;
}

/** Response from GET /posts/json */
export interface ListPostsResponse {
  posts: PostSummary[];
  total: number;
}
