use std::env;
mod compile;
mod graph;
mod runner;
mod runtime;

use graph::resolve;
use graph::DependencyGraph;
use runtime::Runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        panic!("no args");
    }

    let current_dir = env::current_dir()?.to_string_lossy().to_string();
    let entry = &args[1];

    println!("");
    Runtime::from(DependencyGraph::from(entry, &current_dir).await?)
        .run(&resolve(entry, &current_dir))
        .await
}
