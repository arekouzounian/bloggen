use bgc::{parse_markdown_file, Blake3Hash, CasDocument, Client, MarkdownRenderer};
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

    /// Compute delta between two markdown files
    Delta {
        /// Old markdown file
        old: PathBuf,

        /// New markdown file
        new: PathBuf,

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

        /// Show statistics about the delta
        #[arg(short, long)]
        stats: bool,
    },

    /// Upload a markdown file to the server as a new post
    Upload {
        /// Input markdown file to upload
        input: PathBuf,

        /// Post slug (unique identifier)
        #[arg(short, long)]
        slug: String,

        /// Post title (optional)
        #[arg(short, long)]
        title: Option<String>,

        /// Server URL (e.g., http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        server: String,

        /// Show upload statistics
        #[arg(long)]
        stats: bool,
    },

    /// Download a post from the server and optionally render to markdown
    Download {
        /// Post slug to download
        slug: String,

        /// Output file (defaults to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Server URL (e.g., http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        server: String,

        /// Show download statistics
        #[arg(long)]
        stats: bool,
    },

    /// Update a post on the server using delta update
    Update {
        /// Current markdown file (must match server's current version)
        old: PathBuf,

        /// New markdown file with changes
        new: PathBuf,

        /// Post slug to update
        #[arg(short, long)]
        slug: String,

        /// Server URL (e.g., http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        server: String,

        /// Show update statistics
        #[arg(long)]
        stats: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

        Commands::Delta {
            old,
            new,
            output,
            pretty,
            msgpack,
            compress,
            stats,
        } => {
            // Verify input files exist
            if !old.exists() {
                eprintln!("Error: Old file does not exist: {}", old.display());
                std::process::exit(1);
            }
            if !new.exists() {
                eprintln!("Error: New file does not exist: {}", new.display());
                std::process::exit(1);
            }

            // Get file sizes
            let old_size = std::fs::metadata(&old)?.len();
            let new_size = std::fs::metadata(&new)?.len();

            // Parse both markdown files
            let (old_root, old_store) = parse_markdown_file(&old)?;
            let (new_root, new_store) = parse_markdown_file(&new)?;

            // Create CasDocuments
            let old_doc = CasDocument::new(&old_store, old_root)
                .ok_or("Failed to create old CAS document")?;
            let new_doc = CasDocument::new(&new_store, new_root)
                .ok_or("Failed to create new CAS document")?;

            // Compute delta
            let delta = old_doc.compute_delta(&new_doc);
            let delta_stats = delta.stats();

            // Generate output
            let (output_data, format_name) = if msgpack {
                if compress {
                    let data = delta.to_msgpack_compressed()?;
                    (data, "MessagePack+zstd")
                } else {
                    let data = delta.to_msgpack()
                        .map_err(|e| format!("MessagePack serialization failed: {}", e))?;
                    (data, "MessagePack")
                }
            } else {
                let json = if pretty {
                    delta.to_json_pretty()?
                } else {
                    delta.to_json()?
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
                eprintln!("✓ Delta computed successfully");
                eprintln!("  Old file: {} ({} bytes, {} nodes)",
                    old.display(), old_size, old_doc.nodes.len());
                eprintln!("  New file: {} ({} bytes, {} nodes)",
                    new.display(), new_size, new_doc.nodes.len());
                eprintln!("  Old root hash: {}", format_hash(delta.old_root));
                eprintln!("  New root hash: {}", format_hash(delta.new_root));
                eprintln!("  Nodes added: {}", delta_stats.added_count);
                eprintln!("  Nodes removed: {}", delta_stats.removed_count);
                eprintln!("  Root changed: {}", delta_stats.root_changed);
                eprintln!("  Delta size: {} bytes ({})", output_size, format_name);

                // Calculate efficiency
                let full_new_size = if msgpack {
                    if compress {
                        new_doc.to_msgpack_compressed()?.len()
                    } else {
                        new_doc.to_msgpack()?.len()
                    }
                } else {
                    let json = if pretty {
                        new_doc.to_json_pretty()?
                    } else {
                        new_doc.to_json()?
                    };
                    if compress {
                        zstd::bulk::compress(json.as_bytes(), 3)?.len()
                    } else {
                        json.len()
                    }
                };

                let efficiency = 100.0 * (1.0 - (output_size as f64 / full_new_size as f64));
                eprintln!("  Efficiency: {:.1}% smaller than full document", efficiency);
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

        Commands::Upload {
            input,
            slug,
            title,
            server,
            stats,
        } => {
            // Verify input file exists
            if !input.exists() {
                eprintln!("Error: Input file does not exist: {}", input.display());
                std::process::exit(1);
            }

            let input_size = std::fs::metadata(&input)?.len();

            // Parse the markdown file
            let (root_hash, store) = parse_markdown_file(&input)?;

            // Create serializable document
            let doc = CasDocument::new(&store, root_hash)
                .ok_or("Failed to create CAS document from store")?;

            if stats {
                eprintln!("Uploading post to {}...", server);
                eprintln!("  Slug: {}", slug);
                eprintln!("  Title: {}", title.as_deref().unwrap_or("<none>"));
                eprintln!("  Root hash: {}", format_hash(root_hash));
                eprintln!("  Total nodes: {}", doc.nodes.len());
                eprintln!("  Input size: {} bytes", input_size);
            }

            // Create HTTP client and upload
            let client = Client::new(&server);
            let response = client
                .upload_post(&slug, title.as_deref(), doc.root_hash, doc.nodes)
                .await?;

            if stats {
                eprintln!("✓ Upload successful!");
                eprintln!("  Server slug: {}", response.slug);
                eprintln!("  Server root: {}", format_hash(response.ast_root));
                eprintln!("  Created at: {}", response.created_at);
            } else {
                println!("✓ Post uploaded: {}", slug);
            }

            Ok(())
        }

        Commands::Download {
            slug,
            output,
            server,
            stats,
        } => {
            if stats {
                eprintln!("Downloading post from {}...", server);
                eprintln!("  Slug: {}", slug);
            }

            // Create HTTP client and download
            let client = Client::new(&server);
            let response = client.download_post(&slug).await?;

            if stats {
                eprintln!("✓ Download successful!");
                eprintln!("  Root hash: {}", format_hash(response.root_hash));
                eprintln!("  Total nodes: {}", response.nodes.len());
            }

            // Convert to CasDocument
            let doc = CasDocument {
                root_hash: response.root_hash,
                nodes: response.nodes,
            };

            // Convert to NodeStore
            let store = doc.to_store();
            let root = store
                .get(&doc.root_hash)
                .ok_or("Root node not found in document")?;

            // Render to markdown
            let mut renderer = MarkdownRenderer::new(&store);
            let markdown = renderer.render(root)?;

            if stats {
                eprintln!("✓ Rendered to markdown");
                eprintln!("  Output size: {} bytes", markdown.len());
            }

            // Write output
            match output {
                Some(output_path) => {
                    std::fs::write(&output_path, &markdown)?;
                    if stats {
                        eprintln!("✓ Written to: {}", output_path.display());
                    } else {
                        println!("✓ Downloaded to: {}", output_path.display());
                    }
                }
                None => {
                    if stats {
                        eprintln!("---");
                    }
                    print!("{}", markdown);
                }
            }

            Ok(())
        }

        Commands::Update {
            old,
            new,
            slug,
            server,
            stats,
        } => {
            // Verify input files exist
            if !old.exists() {
                eprintln!("Error: Old file does not exist: {}", old.display());
                std::process::exit(1);
            }
            if !new.exists() {
                eprintln!("Error: New file does not exist: {}", new.display());
                std::process::exit(1);
            }

            // Parse both markdown files
            let (old_root, old_store) = parse_markdown_file(&old)?;
            let (new_root, new_store) = parse_markdown_file(&new)?;

            // Create CasDocuments
            let old_doc = CasDocument::new(&old_store, old_root)
                .ok_or("Failed to create old CAS document")?;
            let new_doc = CasDocument::new(&new_store, new_root)
                .ok_or("Failed to create new CAS document")?;

            // Compute delta
            let delta = old_doc.compute_delta(&new_doc);
            let delta_stats_local = delta.stats();

            if stats {
                eprintln!("Updating post on {}...", server);
                eprintln!("  Slug: {}", slug);
                eprintln!("  Old root: {}", format_hash(delta.old_root));
                eprintln!("  New root: {}", format_hash(delta.new_root));
                eprintln!("  Nodes added: {}", delta_stats_local.added_count);
                eprintln!("  Nodes removed: {}", delta_stats_local.removed_count);
            }

            // Create HTTP client and send delta update
            let client = Client::new(&server);
            let response = client
                .update_post_delta(
                    &slug,
                    delta.old_root,
                    delta.new_root,
                    delta.added_nodes,
                    delta.removed_hashes,
                )
                .await?;

            if stats {
                eprintln!("✓ Update successful!");
                eprintln!("  Server slug: {}", response.slug);
                eprintln!("  Server confirmed old root: {}", format_hash(response.old_root));
                eprintln!("  Server confirmed new root: {}", format_hash(response.new_root));
                eprintln!("  Server nodes added: {}", response.nodes_added);
                eprintln!("  Server nodes removed: {}", response.nodes_removed);
            } else {
                println!("✓ Post updated: {}", slug);
            }

            Ok(())
        }
    }
}

fn format_hash(hash: Blake3Hash) -> String {
    hash.to_hex()
}
