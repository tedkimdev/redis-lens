use redis::AsyncCommands;
use std::collections::HashMap;

pub struct KeyInfo {
    pub name: String,
    pub ttl_ms: i64,
}

pub struct ExpiryBucket {
    pub window_start_sec: u64,
    pub count: usize,
    pub keys: Vec<String>,
}

pub async fn scan_keys(
    con: &mut redis::aio::MultiplexedConnection,
    sample_rate: f64,
) -> Result<Vec<KeyInfo>, Box<dyn std::error::Error>> {
    let mut cursor: u64 = 0;
    let mut results: Vec<KeyInfo> = Vec::new();

    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("COUNT")
            .arg(100)
            .query_async(con)
            .await?;

        for key in &keys {
            // skip based on sample rate
            if rand::random::<f64>() > sample_rate {
                continue;
            }

            let ttl_ms: i64 = redis::cmd("PTTL")
                .arg(key)
                .query_async(con)
                .await?;

            results.push(KeyInfo {
                name: key.clone(),
                ttl_ms,
            });
        }

        cursor = next_cursor;
        if cursor == 0 {
            break;
        }
    }

    Ok(results)
}

pub fn analyze_expiry(keys: &[KeyInfo], bucket_size_sec: u64) -> Vec<ExpiryBucket> {
    let mut buckets: HashMap<u64, ExpiryBucket> = HashMap::new();

    for key in keys {
        match key.ttl_ms {
            ms if ms > 0 => {
                let sec = (ms / 1000) as u64;
                let window = (sec / bucket_size_sec) * bucket_size_sec;
                let bucket = buckets.entry(window).or_insert(ExpiryBucket {
                    window_start_sec: window,
                    count: 0,
                    keys: Vec::new(),
                });
                bucket.count += 1;
                bucket.keys.push(key.name.clone());
            }
            _ => {}
        }
    }

    let mut result: Vec<ExpiryBucket> = buckets.into_values().collect();
    result.sort_by_key(|b| b.window_start_sec);
    result
}

pub fn risk_score(buckets: &[ExpiryBucket], total_keys: usize) -> u8 {
    if total_keys == 0 {
        return 0;
    }

    // find the bucket with the most keys
    let max_count = buckets.iter().map(|b| b.count).max().unwrap_or(0);

    // what % of total keys expire in the busiest window
    let score = (max_count * 100) / total_keys;

    // cap at 100 and cast to u8
    score.min(100) as u8
}
