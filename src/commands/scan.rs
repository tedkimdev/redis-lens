use crate::scanner;
use colored::Colorize;

pub async fn run(
    con: &mut redis::aio::MultiplexedConnection,
    bucket: u64,
    sample: f64,
    pattern: Option<&str>,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let keys = scanner::scan_keys(con, sample, pattern).await?;
    let buckets = scanner::analyze_expiry(&keys, bucket);
    let score = scanner::risk_score(&buckets, keys.len());

    if output == "json" {
        let out = serde_json::json!({
            "risk_score": score,
            "total_keys": keys.len(),
            "buckets": buckets.iter().map(|b| serde_json::json!({
                "window_start_sec": b.window_start_sec,
                "count": b.count,
                "keys": b.keys,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let max_count = buckets.iter().map(|b| b.count).max().unwrap_or(1);

    println!("Expiry Distribution: \n");
    for bucket_item in &buckets {
        let bar = "█".repeat(bucket_item.count * 20 / max_count);
        let is_riskiest = bucket_item.count == max_count && max_count > 1;

        let line = format!(
            "  {:^6}~{:^6}s  {:<20} {} keys",
            bucket_item.window_start_sec,
            bucket_item.window_start_sec + bucket,
            bar,
            bucket_item.count,
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
    print_recommendation(score, &buckets, bucket);
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