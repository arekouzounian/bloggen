use bgc::{parse_markdown_file, Blake3Hash, CasDocument, MarkdownRenderer};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bgc")]
#[command(about = "BlogGen Client - Content-Addressable Storage for Markdown", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a markdown file into JSON or MessagePack AST format
    Parse {
        /// Input markdown file to parse
        input: PathBuf,

        /// Output file (defaults to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Use pretty-printed JSON format
        #[arg(short, long)]
        pretty: bool,

        /// Output MessagePack binary format instead of JSON
        #[arg(short, long)]
        msgpack: bool,

        /// Compress output with zstd (works with both JSON and MessagePack)
        #[arg(short, long)]
        compress: bool,

        /// Show statistics about the parsed document
        #[arg(short, long)]
        stats: bool,
    },

    /// Render a CAS document (JSON/MessagePack) back to markdown
    Render {
        /// Input CAS document file (JSON or MessagePack)
        input: PathBuf,

        /// Output file (defaults to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Input is MessagePack format
        #[arg(short, long)]
        msgpack: bool,

        /// Input is compressed with zstd
        #[arg(short, long)]
        compressed: bool,

        /// Show statistics about the rendering
        #[arg(short, long)]
        stats: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            input,
            output,
            pretty,
            msgpack,
            compress,
            stats,
        } => {
            // Verify input file exists
            if !input.exists() {
                eprintln!("Error: Input file does not exist: {}", input.display());
                std::process::exit(1);
            }

            // Get input file size
            let input_size = std::fs::metadata(&input)?.len();

            // Parse the markdown file
            let (root_hash, store) = parse_markdown_file(&input)?;

            // Create serializable document
            let doc = CasDocument::new(&store, root_hash)
                .ok_or("Failed to create CAS document from store")?;

            // Generate output
            let (output_data, format_name) = if msgpack {
                if compress {
                    let data = doc.to_msgpack_compressed()?;
                    (data, "MessagePack+zstd")
                } else {
                    let data = doc.to_msgpack()
                        .map_err(|e| format!("MessagePack serialization failed: {}", e))?;
                    (data, "MessagePack")
                }
            } else {
                let json = if pretty {
                    doc.to_json_pretty()?
                } else {
                    doc.to_json()?
                };
                let json_bytes = json.into_bytes();
                if compress {
                    let compressed = zstd::bulk::compress(&json_bytes, 3)?;
                    (compressed, "JSON+zstd")
                } else {
                    (json_bytes, "JSON")
                }
            };

            let output_size = output_data.len();

            if stats {
                eprintln!("✓ Parsed successfully");
                eprintln!("  Root hash: {}", format_hash(root_hash));
                eprintln!("  Total nodes stored: {}", store.len());
                eprintln!("  Input size: {} bytes", input_size);
                eprintln!("  Output size: {} bytes ({})", output_size, format_name);
                let ratio = output_size as f64 / input_size as f64;
                eprintln!("  Size ratio: {:.2}x", ratio);
            }

            // Write output
            match output {
                Some(output_path) => {
                    std::fs::write(&output_path, &output_data)?;
                    if stats {
                        eprintln!("✓ Written to: {}", output_path.display());
                    }
                }
                None => {
                    // Write to stdout
                    if msgpack {
                        // Binary output to stdout
                        use std::io::Write;
                        std::io::stdout().write_all(&output_data)?;
                    } else {
                        // Text output to stdout
                        println!("{}", String::from_utf8_lossy(&output_data));
                    }
                }
            }

            Ok(())
        }

        Commands::Render {
            input,
            output,
            msgpack,
            compressed,
            stats,
        } => {
            // Verify input file exists
            if !input.exists() {
                eprintln!("Error: Input file does not exist: {}", input.display());
                std::process::exit(1);
            }

            // Get input file size
            let input_size = std::fs::metadata(&input)?.len();

            // Read input file
            let input_data = std::fs::read(&input)?;

            // Determine format from flags or file extension
            let format_name = if compressed {
                if msgpack {
                    "MessagePack+zstd"
                } else {
                    "JSON+zstd"
                }
            } else if msgpack {
                "MessagePack"
            } else {
                "JSON"
            };

            // Deserialize document
            let doc = if compressed {
                if msgpack {
                    CasDocument::from_msgpack_compressed(&input_data)?
                } else {
                    // Decompress then parse JSON
                    let decompressed = zstd::bulk::decompress(&input_data, 10_000_000)?;
                    let json_str = String::from_utf8(decompressed)
                        .map_err(|e| format!("Invalid UTF-8 in decompressed JSON: {}", e))?;
                    CasDocument::from_json(&json_str)?
                }
            } else if msgpack {
                CasDocument::from_msgpack(&input_data)?
            } else {
                let json_str = String::from_utf8(input_data)
                    .map_err(|e| format!("Invalid UTF-8 in JSON file: {}", e))?;
                CasDocument::from_json(&json_str)?
            };

            if stats {
                eprintln!("✓ Loaded CAS document ({})", format_name);
                eprintln!("  Root hash: {}", format_hash(doc.root_hash));
                eprintln!("  Total nodes: {}", doc.nodes.len());
                eprintln!("  Input size: {} bytes", input_size);
            }

            // Convert document into NodeStore and get root
            let store = doc.to_store();
            let root = store.get(&doc.root_hash)
                .ok_or("Root node not found in document")?;

            // Render to markdown
            let mut renderer = MarkdownRenderer::new(&store);
            let markdown = renderer.render(root)?;

            if stats {
                eprintln!("✓ Rendered to markdown");
                eprintln!("  Output size: {} bytes", markdown.len());
                let ratio = markdown.len() as f64 / input_size as f64;
                eprintln!("  Size ratio: {:.2}x", ratio);
            }

            // Write output
            match output {
                Some(output_path) => {
                    std::fs::write(&output_path, &markdown)?;
                    if stats {
                        eprintln!("✓ Written to: {}", output_path.display());
                    }
                }
                None => {
                    print!("{}", markdown);
                }
            }

            Ok(())
        }
    }
}

fn format_hash(hash: Blake3Hash) -> String {
    hash.to_hex()
}
