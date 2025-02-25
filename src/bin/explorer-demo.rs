//! # explorer-demo
//! This is a minimal application that demonstrates how to use the `cgroups_explorer` crate.

use cgroups_explorer::Explorer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<String>>();
    let mut builder = Explorer::detect_version();
    println!("Using filters: {:?}", &args[1..]);
    if args.len() > 1 {
        builder = builder.include(args[1..].to_vec());
    }
    let explorer = builder.build()?;
    for cgroup in explorer.iter_cgroups() {
        let memory_usage =
            if let Some(memory) = cgroup.controller_of::<cgroups_rs::memory::MemController>() {
                memory.memory_stat().usage_in_bytes
            } else {
                0
            };
        println!(
            "Detected cgroup {}, mem usage {}",
            cgroup.path(),
            memory_usage
        );
    }

    Ok(())
}
