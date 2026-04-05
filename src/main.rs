mod scanner;
mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "redis-lens", about = "Redis stampede risk analyzer")]
struct Args {
    /// Redis connection URL (or set REDIS_URL env var)
    #[arg(long, env = "REDIS_URL", default_value = "redis://127.0.0.1/")]
    url: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Anlayze key expiry distribution and stampede risk
    Scan {
        /// Bucket size in seconds
        #[arg(long, default_value_t = 60)]
        bucket: u64,

        /// Sample rate (0.0 ~ 1.0)
        #[arg(long, default_value_t = 1.0)]
        sample: f64,

        #[arg(long, default_value = "text")]
        output: String,

        /// Only scan keys matching this pattern (e.g. user:*)
        #[arg(long)]
        pattern: Option<String>,
    },
    /// Analyze memory usage by key pattern
    Memory {
        #[arg(long, default_value_t = 1.0)]
        sample: f64,

        #[arg(long, default_value = "text")]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let client = redis::Client::open(args.url)?;
    let mut con = client.get_multiplexed_async_connection().await?;
    
    match args.command {
        Command::Scan { bucket, sample, output, pattern } => {
            commands::scan::run(&mut con, bucket, sample, pattern.as_deref(), &output).await?;
        },
        Command::Memory { sample, output } => {
            commands::memory::run(&mut con, sample, &output).await?;
        }
    }

    Ok(())
}
