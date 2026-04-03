mod scanner;

use clap::Parser;

#[derive(Parser)]
#[command(name = "redis-lens", about = "Redis stampede risk analyzer")]
struct Args {
    /// Redis connection URL
    #[arg(long, default_value = "redis://127.0.0.1/")]
    url: String,

    /// Bucket size in seconds
    #[arg(long, default_value_t = 60)]
    bucekt: u64,

    /// Sample rate (0.0 ~ 1.0)
    #[arg(long, default_value_t = 1.0)]
    sample: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let client = redis::Client::open(args.url)?;
    let mut con = client.get_multiplexed_async_connection().await?;
    
    let keys = scanner::scan_keys(&mut con).await?;
    let buckets = scanner::analyze_expiry(&keys, 60);
    let score = scanner::risk_score(&buckets, keys.len());

    let max_count = buckets.iter().map(|b| b.count).max().unwrap_or(1);

    println!("Expiry Distribution: \n");
    for bucket in &buckets {
        let bar = "█".repeat(bucket.count * 20 / max_count);
        let risk = if bucket.count == max_count && max_count > 1 {
            " ⚠ HIGH RISK"
        } else {
            ""
        };
        println!(
            "  {:^6}~{:^6}s  {:<20} {} keys{}",
            bucket.window_start_sec,
            bucket.window_start_sec + 60,
            bar,
            bucket.count,
            risk
        );
    }
    
    println!("\nRisk Score: {}/100", score);
    println!("total number of keys: {}", keys.len());
    Ok(())
}
