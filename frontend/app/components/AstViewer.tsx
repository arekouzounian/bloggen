import type { AstNode, Blake3Hash, CasDocumentResponse } from "@/app/types/ast";

interface AstViewerProps {
  doc: CasDocumentResponse;
}

interface NodeTreeProps {
  hash: Blake3Hash;
  nodes: Record<Blake3Hash, AstNode>;
  depth?: number;
}

/**
 * Recursively renders a single CAS AST node as an indented tree.
 *
 * This is the "proof of concept" renderer — it visualizes the tree structure
 * of the AST. See frontend/README.md for a discussion of production rendering
 * approaches.
 */
function NodeTree({ hash, nodes, depth = 0 }: NodeTreeProps) {
  const node = nodes[hash];
  const indent = depth * 16;

  if (!node) {
    return (
      <div style={{ paddingLeft: indent }} className="text-red-500 text-xs">
        ⚠ Missing node: <code className="font-mono">{hash.slice(0, 12)}…</code>
      </div>
    );
  }

  const shortHash = hash.slice(0, 8);
  const children =
    "children" in node && Array.isArray(node.children) ? node.children : [];

  return (
    <div style={{ paddingLeft: indent }} className="my-0.5">
      {/* Node header */}
      <div className="flex items-baseline gap-2">
        <span className="font-mono text-xs text-gray-400">{shortHash}…</span>
        <NodeSummary node={node} />
      </div>
      {/* Recurse into children */}
      {children.map((childHash) => (
        <NodeTree
          key={childHash}
          hash={childHash}
          nodes={nodes}
          depth={depth + 1}
        />
      ))}
    </div>
  );
}

/** Renders a concise one-line summary of a single AST node */
function NodeSummary({ node }: { node: AstNode }) {
  switch (node.type) {
    case "root":
      return (
        <span className="text-purple-700 font-semibold">
          Root ({node.children.length} children)
        </span>
      );
    case "heading":
      return (
        <span className="text-blue-700 font-semibold">
          H{node.level} ({node.children.length} children)
        </span>
      );
    case "paragraph":
      return (
        <span className="text-blue-600">
          Paragraph ({node.children.length} children)
        </span>
      );
    case "text":
      return (
        <span className="text-green-700">
          Text: <em className="text-gray-700 not-italic">&ldquo;{truncate(node.value, 60)}&rdquo;</em>
        </span>
      );
    case "strong":
      return <span className="font-bold text-gray-800">Strong</span>;
    case "emphasis":
      return <span className="italic text-gray-700">Emphasis</span>;
    case "delete":
      return <span className="line-through text-gray-600">Delete</span>;
    case "code_block":
      return (
        <span className="text-yellow-700 font-mono">
          CodeBlock{node.lang ? ` (${node.lang})` : ""}:{" "}
          <em className="not-italic text-xs text-gray-500">{truncate(node.value, 40)}</em>
        </span>
      );
    case "inline_code":
      return (
        <span className="text-yellow-600 font-mono">
          InlineCode: <code>{truncate(node.value, 40)}</code>
        </span>
      );
    case "link":
      return (
        <span className="text-blue-500 underline">
          Link: {node.url}
        </span>
      );
    case "image":
      return (
        <span className="text-teal-600">
          Image: {node.alt} ({node.url})
        </span>
      );
    case "list":
      return (
        <span className="text-indigo-700">
          {node.ordered ? "OrderedList" : "UnorderedList"} ({node.children.length} items)
        </span>
      );
    case "list_item":
      return (
        <span className="text-indigo-600">
          ListItem
          {node.checked === true
            ? " [x]"
            : node.checked === false
              ? " [ ]"
              : ""}
        </span>
      );
    case "blockquote":
      return <span className="text-gray-600 italic">Blockquote</span>;
    case "thematic_break":
      return <span className="text-gray-400">ThematicBreak (---)</span>;
    case "table":
      return <span className="text-cyan-700">Table ({node.children.length} rows)</span>;
    case "table_row":
      return <span className="text-cyan-600">TableRow</span>;
    case "table_cell":
      return <span className="text-cyan-500">TableCell</span>;
    case "html":
      return (
        <span className="text-orange-600 font-mono">
          HTML: {truncate(node.value, 40)}
        </span>
      );
    case "yaml":
      return (
        <span className="text-pink-600 font-mono">
          YAML frontmatter: {truncate(node.value, 40)}
        </span>
      );
    case "toml":
      return (
        <span className="text-pink-500 font-mono">
          TOML frontmatter: {truncate(node.value, 40)}
        </span>
      );
    case "break":
      return <span className="text-gray-400">LineBreak</span>;
    case "definition":
      return <span className="text-gray-500">Definition [{node.identifier}]: {node.url}</span>;
    case "link_reference":
      return <span className="text-blue-400">LinkReference [{node.identifier}]</span>;
    case "image_reference":
      return <span className="text-teal-400">ImageReference [{node.identifier}]</span>;
    case "footnote_definition":
      return <span className="text-gray-500">FootnoteDef [{node.identifier}]</span>;
    case "footnote_reference":
      return <span className="text-gray-400">FootnoteRef [{node.identifier}]</span>;
    case "math":
      return <span className="text-red-600 font-mono">Math: {truncate(node.value, 40)}</span>;
    case "inline_math":
      return <span className="text-red-500 font-mono">InlineMath: {truncate(node.value, 30)}</span>;
    default:
      return (
        <span className="text-gray-400 text-xs font-mono">
          {(node as AstNode).type}
        </span>
      );
  }
}

function truncate(s: string, n: number): string {
  if (s.length <= n) return s;
  return s.slice(0, n) + "…";
}

/**
 * Displays the full CAS AST document as an interactive tree view.
 *
 * This is a proof-of-concept renderer that shows the raw AST structure.
 * For a production rendering implementation, see the README for a discussion
 * of the different rendering approach trade-offs.
 */
export function AstViewer({ doc }: AstViewerProps) {
  const nodeCount = Object.keys(doc.nodes).length;

  return (
    <div className="space-y-4">
      {/* Document metadata */}
      <div className="bg-gray-50 rounded p-3 text-xs font-mono border">
        <p>
          <span className="text-gray-500">root_hash: </span>
          <span className="text-purple-600">{doc.root_hash}</span>
        </p>
        <p>
          <span className="text-gray-500">total nodes: </span>
          <span className="text-blue-600">{nodeCount}</span>
        </p>
      </div>

      {/* AST tree view */}
      <div className="bg-white border rounded p-4 overflow-x-auto text-sm leading-relaxed">
        <NodeTree hash={doc.root_hash} nodes={doc.nodes} />
      </div>

      {/* Raw JSON dump (collapsible) */}
      <details className="border rounded">
        <summary className="cursor-pointer px-4 py-2 bg-gray-50 text-sm font-medium text-gray-700 hover:bg-gray-100">
          Raw JSON response
        </summary>
        <pre className="p-4 text-xs font-mono bg-gray-900 text-gray-100 overflow-x-auto rounded-b max-h-[600px] overflow-y-auto">
          {JSON.stringify(doc, null, 2)}
        </pre>
      </details>
    </div>
  );
}
