#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let mut con = client.get_multiplexed_async_connection().await?;

    for i in 0..10000u64 {
        let ttl = rand::random::<u64>() % 3600 + 1;
        let key = format!("user:{}:profile", i);
        redis::cmd("SET")
            .arg(&key)
            .arg("data")
            .arg("EX")
            .arg(ttl)
            .query_async::<_, ()>(&mut con)
            .await?;

        if i % 1000 == 0 {
            println!("populated {}/10000", i);
        }
    }

    println!("done!");
    Ok(())
}