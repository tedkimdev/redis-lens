use std::collections::HashMap;

use colored::Colorize;

pub async fn run(
    con: &mut redis::aio::MultiplexedConnection,
    sample: f64,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // get server memory info
    let info: String = redis::cmd("INFO").arg("memory").query_async(con).await?;

    let used_memory = parse_info(&info, "used_memory").unwrap_or(0);
    let max_memory = parse_info(&info, "maxmemory").unwrap_or(0);
    let frag_ratio = parse_info_float(&info, "mem_fragmentation_ratio").unwrap_or(1.0);

    // scan keys and get memory usage per pattern
    let mut cursor: u64 = 0;
    let mut pattern_memory: HashMap<String, (u64, usize)> = HashMap::new(); // (total_bytes, count)

    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("COUNT")
            .arg(100)
            .query_async(con)
            .await?;

        // pipeline MEMORY USAGE for batch
        let sampled: Vec<&String> = keys
            .iter()
            .filter(|_| rand::random::<f64>() <= sample)
            .collect();

        if !sampled.is_empty() {
            let mut pipe = redis::pipe();
            for key in &sampled {
                pipe.cmd("MEMORY").arg("USAGE").arg(key);
            }
            let sizes: Vec<Option<u64>> = pipe.query_async(con).await?;

            for (key, size) in sampled.iter().zip(sizes.iter()) {
                if let Some(bytes) = size {
                    let prefix = if key.contains(':') {
                        key.split(':').next().unwrap_or(key).to_string()
                    } else {
                        "(no namespace)".to_string()
                    };
                    let entry = pattern_memory.entry(prefix).or_insert((0, 0));
                    entry.0 += bytes;
                    entry.1 += 1;
                }
            }
        }

        cursor = next_cursor;
        if cursor == 0 {
            break;
        }
    }

    if output == "json" {
        let out = serde_json::json!({
            "used_memory_bytes": used_memory,
            "max_memory_bytes": max_memory,
            "fragmentation_ratio": frag_ratio,
            "patterns": pattern_memory.iter().map(|(k, (bytes, count))| {
                serde_json::json!({
                    "prefix": k,
                    "total_bytes": bytes,
                    "key_count": count,
                    "avg_bytes": *bytes / ((*count as u64).max(1)),
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // sort by total memory desc
    let mut patterns: Vec<(&String, &(u64, usize))> = pattern_memory.iter().collect();
    patterns.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    let max_bytes = patterns.first().map(|(_, (b, _))| *b).unwrap_or(1);

    println!("Memory Usage by Pattern:\n");
    for (prefix, (total_bytes, count)) in &patterns {
        let bar = "█".repeat((total_bytes * 20 / max_bytes) as usize);
        let avg = *total_bytes / (*count as u64).max(1);
        let display = if *prefix == "(no namespace)" {
            "(no namespace)".to_string()
        } else {
            format!("{}:*", prefix)
        };

        println!(
            "  {:<14} {:<20} {} ({} keys, avg {})",
            display,
            bar,
            format_bytes(*total_bytes),
            count,
            format_bytes(avg),
        );
    }

    // server memory
    println!();
    if max_memory > 0 {
        let pct = used_memory * 100 / max_memory;
        let mem_line = format!(
            "Server Memory: {} / {} ({}%)",
            format_bytes(used_memory),
            format_bytes(max_memory),
            pct,
        );
        match pct {
            0..=79 => println!("{}", mem_line.green()),
            80..=89 => println!("{}", mem_line.yellow()),
            _ => println!("{}", mem_line.red().bold()),
        }
    } else {
        println!(
            "Server Memory: {} (no limit set)",
            format_bytes(used_memory)
        );
    }

    // tips
    print_tips(&patterns, used_memory, max_memory, frag_ratio);

    Ok(())
}

fn print_tips(
    patterns: &[(&String, &(u64, usize))],
    used_memory: u64,
    max_memory: u64,
    frag_ratio: f32,
) {
    println!("\nTips:");

    let mut has_tips = false;

    // high avg memory per key
    for (prefix, (total, count)) in patterns {
        let avg = *total / (*count as u64).max(1);
        if avg > 10_000 {
            println!(
                "  {} {}:* avg {} per key is high",
                "⚠".yellow(),
                prefix,
                format_bytes(avg),
            );
            println!("    → consider compressing values with msgpack or bincode");
            println!("    → or split into smaller keys");
            has_tips = true;
        }
    }

    // high memory usage
    if max_memory > 0 {
        let pct = used_memory * 100 / max_memory;
        if pct >= 90 {
            println!(
                "  {} memory usage at {}% — consider upgrading your Redis plan",
                "✗".red(),
                pct
            );
            has_tips = true;
        } else if pct >= 80 {
            println!(
                "  {} memory usage at {}% — monitor closely",
                "⚠".yellow(),
                pct
            );
            has_tips = true;
        }
    }

    // high fragmentation
    if frag_ratio > 1.5 {
        println!(
            "  {} fragmentation ratio {:.1} is high",
            "⚠".yellow(),
            frag_ratio
        );
        println!("    → run MEMORY PURGE to defragment");
        has_tips = true;
    }

    if !has_tips {
        println!("  {} memory usage looks healthy", "✓".green());
    }
}

fn format_bytes(bytes: u64) -> String {
    match bytes {
        b if b >= 1_000_000 => format!("{:.1}MB", b as f64 / 1_000_000.0),
        b if b >= 1_000 => format!("{:.1}KB", b as f64 / 1_000.0),
        b => format!("{}B", b),
    }
}

fn parse_info(info: &str, key: &str) -> Option<u64> {
    info.lines()
        .find(|l| l.starts_with(key))?
        .split(':')
        .nth(1)?
        .trim()
        .parse()
        .ok()
}

fn parse_info_float(info: &str, key: &str) -> Option<f32> {
    info.lines()
        .find(|l| l.starts_with(key))?
        .split(':')
        .nth(1)?
        .trim()
        .parse()
        .ok()
}
