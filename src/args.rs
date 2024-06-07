use clap::Parser;

/// Tool for exploring RocksDB guarantees
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Args {
    /// Working directory for Qdrant data
    #[arg(long, default_value = "storage")]
    pub storage_dir: String,
    /// Configure the flush interval
    #[arg(long, default_value_t = 1000)]
    pub flush_interval_ms: usize,
    /// Whether to enable RocksDB WAL
    #[arg(long, default_value_t = false)]
    pub wal_enabled: bool,
}
