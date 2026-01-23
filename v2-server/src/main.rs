use axum::{
    routing::{delete, get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use v2_server::{config::Config, db::Database, handlers};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let mut config = Config::from_file("config.json")
        .or_else(|_| Config::from_file("/etc/bloggen/config.json"))
        .unwrap_or_else(|_| {
            eprintln!("Warning: Could not load config file, using defaults");
            Config {
                server: v2_server::config::ServerConfig {
                    host: "127.0.0.1".to_string(),
                    port: 3000,
                },
                database: v2_server::config::DatabaseConfig {
                    url: std::env::var("DATABASE_URL")
                        .unwrap_or_else(|_| "postgres://localhost/bloggen".to_string()),
                    max_connections: 10,
                    min_connections: 2,
                },
                logging: Default::default(),
            }
        });

    // Allow DATABASE_URL environment variable to override config file
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        tracing::debug!("Overriding database URL from DATABASE_URL environment variable");
        config.database.url = db_url;
    }

    // Setup logging
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.logging.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    if config.logging.json {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    tracing::info!("Starting BlogGen v2 server");
    tracing::info!("Configuration loaded: {:?}", config);

    // Setup database connection pool
    tracing::info!("Connecting to database: {}", config.database.url);
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&config.database.url)
        .await?;

    tracing::info!("Database connection pool established");

    // Run migrations
    tracing::info!("Running database migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Create database wrapper
    let db = Arc::new(Database::new(pool));

    // Build application with all endpoints
    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/posts", post(handlers::create_post))
        .route("/posts", get(handlers::list_posts))
        .route("/posts/:slug/ast", get(handlers::get_post_ast))
        .route("/posts/:slug/delta", post(handlers::update_post_delta))
        .route("/posts/:slug/markdown", get(handlers::get_post_markdown))
        .route("/posts/:slug", delete(handlers::delete_post))
        .with_state(db);

    // Start server
    let addr = SocketAddr::from((
        config
            .server
            .host
            .parse::<std::net::IpAddr>()
            .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        config.server.port,
    ));

    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C (SIGINT)");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM");
        },
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
