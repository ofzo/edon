use anyhow::bail;
use std::env;
mod builtin;
mod compile;
mod compile_oxc;
mod graph;
mod runtime;
// mod compile_swc;

use graph::resolve;
use graph::DependencyGraph;
use runtime::Runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() <= 1 {
        bail!("no args");
    }

    let current_dir = env::current_dir()?.to_string_lossy().to_string();
    let entry = &args[1];

    // println!("");
    Runtime::from(DependencyGraph::from(entry, &current_dir).await?)
        .run(&resolve(entry, &current_dir))
        .await
}
