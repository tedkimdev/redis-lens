mod scanner;

use clap::Parser;
use colored::Colorize;

#[derive(Parser)]
#[command(name = "redis-lens", about = "Redis stampede risk analyzer")]
struct Args {
    /// Redis connection URL
    #[arg(long, default_value = "redis://127.0.0.1/")]
    url: String,

    /// Bucket size in seconds
    #[arg(long, default_value_t = 60)]
    bucket: u64,

    /// Sample rate (0.0 ~ 1.0)
    #[arg(long, default_value_t = 1.0)]
    sample: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let client = redis::Client::open(args.url)?;
    let mut con = client.get_multiplexed_async_connection().await?;
    
    let keys = scanner::scan_keys(&mut con, args.sample).await?;
    let buckets = scanner::analyze_expiry(&keys, 60);
    let score = scanner::risk_score(&buckets, keys.len());

    let max_count = buckets.iter().map(|b| b.count).max().unwrap_or(1);

    println!("Expiry Distribution: \n");
    for bucket in &buckets {
        let bar = "█".repeat(bucket.count * 20 / max_count);
        let is_riskiest = bucket.count == max_count && max_count > 1;

        let line = format!(
            "  {:^6}~{:^6}s  {:<20} {} keys",
            bucket.window_start_sec,
            bucket.window_start_sec + args.bucket,
            bar,
            bucket.count,
        );
        if is_riskiest {
            println!("{} {}", line.red(), "⚠ HIGH RISK".red().bold());
        } else {
            println!("{}", line);
        }
    }

    let score_str = format!("\nRisk Score: {}/100", score);
    match score {
        0..=30 => println!("{}", score_str.green()),
        31..=60 => println!("{}", score_str.yellow()),
        _ => println!("{}", score_str.red().bold()),
    }
    
    println!("Total keys scanned: {}", keys.len());
    print_recommendation(score, &buckets, args.bucket);
    Ok(())
}

fn print_recommendation(score: u8, buckets: &[scanner::ExpiryBucket], bucket_size: u64) {
    println!();
    if score == 0 {
        println!("{}", "✓ No expiring keys found — no risk detected".green());
        return;
    }

    // find the riskiest window
    let Some(riskiest) = buckets.iter().max_by_key(|b| b.count) else {
        return;
    };

    match score {
        0..=30 => println!("{}", "✓ Low risk — expiry is well distributed".green()),
        31..=60 => println!(
            "{}",
            format!(
                "⚠ Medium risk — {} keys expire in the {}~{}s window\n  Consider adding jitter: TTL + rand(0..{})",
                riskiest.count,
                riskiest.window_start_sec,
                riskiest.window_start_sec + bucket_size,
                bucket_size / 2,
            ).yellow()
        ),
        _ => {
            println!(
                "{}",
                format!(
                    "✗ High risk — {} keys expire in the {}~{}s window",
                    riskiest.count,
                    riskiest.window_start_sec,
                    riskiest.window_start_sec + bucket_size,
                ).red().bold()
            );
            println!("  Recommendation: SET key value EX $((TTL + RANDOM % {}))", bucket_size / 2);
            println!("\n  Affected keys:");
            for key in riskiest.keys.iter().take(10) {
                println!("    {}", key);
            }
            if riskiest.keys.len() > 10 {
                println!("    {} {}", "... and".dimmed(), format!("{} more", riskiest.keys.len() - 10).dimmed());
            }
        },
    }
}