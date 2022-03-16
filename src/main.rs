use std::time::Duration;
// use swc;
use tokio::time::sleep;

use crate::swc::swc_main;

mod executor;
mod swc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = tokio::fs::read("output/test.js").await?;
    let code = swc_main(&String::from_utf8_lossy(&source).to_string());

    tokio::spawn(sleep(Duration::from_micros(1)));

    executor::run(code);

    Ok(())
}
