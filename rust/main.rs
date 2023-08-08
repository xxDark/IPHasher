use std::{
  sync::{
      atomic::{AtomicBool, AtomicU64, Ordering},
      Arc,
  },
  time::Instant,
};

use anyhow::Result;
use sha2::{Digest, Sha256};
use tokio::time::{sleep, Duration};

#[tokio::main]

// take in hash as cmd args
async fn main() -> Result<()> {
  let hash = std::env::args()
      .nth(1)
      .ok_or(anyhow::anyhow!("Please provide a hash as the first argument"))?;
  let hash = hex::decode(hash)?;

  let min_ip = 0x00000000u64;
  let max_ip = 0xffffffffu64;

  let cpus = num_cpus::get();
  let total_ips = max_ip - min_ip + 1;
  let step_size = total_ips / cpus as u64;

  let mut ip = min_ip;
  let mut tasks = vec![];

  let now = Instant::now();
  let processed = Arc::new(AtomicU64::new(0));
  let done = Arc::new(AtomicBool::new(false));

  for _ in 0..cpus {
      let start_ip = ip;
      let end_ip = ip + step_size - 1;

      let hash = hash.clone();
      let processed = processed.clone();
      let done = done.clone();

      let task = tokio::spawn(async move {
          let mut hasher = Sha256::new();

          for ip in start_ip..=end_ip {
              if done.load(Ordering::Relaxed) {
                  break;
              }

              let ip = format!(
                  "{}.{}.{}.{}",
                  ip >> 24 & 0xff,
                  ip >> 16 & 0xff,
                  ip >> 8 & 0xff,
                  ip & 0xff
              );
              hasher.update(ip.clone());

              let result = hasher.finalize_reset();

              if result[..] == hash[..] {
                  println!("Found! IP: {}", ip);
                  done.store(true, Ordering::Relaxed);
                  break;
              }

              processed.fetch_add(1, Ordering::Relaxed);
          }
      });
      tasks.push(task);

      ip += step_size;
  }

  let processed = processed.clone();
  tokio::spawn(async move {
      loop {
          let processed = processed.load(Ordering::Relaxed);
          let ips_per_sec = processed as f64 / now.elapsed().as_secs_f64();

          let progress = processed as f64 / total_ips as f64 * 100.0;
          let remaining_ips = total_ips - processed;
          let est_remaining_secs = remaining_ips as f64 / ips_per_sec;

          println!(
              "Progress: {:.2}%, Speed: {:.2} ips/s, Remaining: {:.2}s",
              progress, ips_per_sec, est_remaining_secs
          );

          if done.load(Ordering::Relaxed) {
              break;
          }

          sleep(Duration::from_millis(100)).await;
      }
  });

  for task in tasks {
      task.await?;
  }

  Ok(())
}