use std::env;
use std::sync::Arc;
use std::time::Instant;

use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::json;
use tokio::sync::Semaphore;

use laravel_passport_introspection::{
    config::Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env(None).map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

    let socket_path = env::var("SOCKET_PATH").unwrap_or_else(|_| "/tmp/introspector.sock".to_string()).trim().to_string();

    config.get_algorithm().expect("Invalid JWT_ALGORITHM in config");
    let alg_str = config.jwt_algorithm;
    let path = format!("wrk/tokens_{}.txt", alg_str);
    let tokens = load_tokens(&path).await;
    if tokens.len() == 0 {
        std::process::exit(1);
    }

    let concurrency = 400;
    let total_requests = 1_000_000;

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..total_requests {
        let token = tokens[i % tokens.len()].clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let socket_path_copy = socket_path.clone();
        let handle = tokio::spawn(async move {
            let result = send_request(&socket_path_copy, &token).await;
            drop(permit);
            result
        });
        handles.push(handle);
    }

    let mut success = 0;
    let mut errors = 0;
    let mut latencies = vec![];

    for handle in handles {
        match handle.await.unwrap() {
            Ok(latency) => {
                success += 1;
                latencies.push(latency);
            }
            Err(_) => errors += 1,
        }
    }

    let elapsed = start.elapsed();
    let rps = total_requests as f64 / elapsed.as_secs_f64();

    println!("=== Unix Socket Benchmark ===");
    println!("Total requests: {}", total_requests);
    println!("Success: {}", success);
    println!("Errors: {}", errors);
    println!("Time: {:?}", elapsed);
    println!("RPS: {:.0}", rps);

    if !latencies.is_empty() {
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("Latency p50: {:.2}ms", latencies[latencies.len()/2] * 1000.0);
        println!("Latency p90: {:.2}ms", latencies[latencies.len()*9/10] * 1000.0);
        println!("Latency p99: {:.2}ms", latencies[latencies.len()*99/100] * 1000.0);
    }

    Ok(())
}

async fn send_request(socket_path: &str, token: &str) -> Result<f64, anyhow::Error> {
    let start = Instant::now();

    let mut stream = UnixStream::connect(&socket_path).await?;

    let request = json!({
        "token": token,
    });

    stream.write_all(request.to_string().as_bytes()).await?;
    stream.flush().await?;

    let mut response = vec![0u8; 4096];
    let n = stream.read(&mut response).await?;

    let _ = String::from_utf8_lossy(&response[..n]);
    let elapsed = start.elapsed().as_secs_f64();

    Ok(elapsed)
}

async fn load_tokens(path: &str) -> Vec<String> {
    let content = tokio::fs::read_to_string(path).await.unwrap_or_default();
    let tokens: Vec<String> = content.lines()
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
        .collect();

    if tokens.len() == 0 {
        eprintln!("⚠️  WARNING: No tokens loaded from {}", path);
        eprintln!("   Generate tokens with: cargo run --bin wrk_access_tokens_factory");
    }

    tokens
}
