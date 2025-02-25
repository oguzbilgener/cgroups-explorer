# cgroups-explorer

![Crates.io Version](https://img.shields.io/crates/v/:cgroups-explorer)

A crate for exploring control groups v1 and v2 on Linux.

Uses the [`cgroups-rs`](https://github.com/kata-containers/cgroups-rs) crate to interact with cgroups.

## Usage

```rust
use cgroups_explorer::Explorer;

let explorer = Explorer::detect_version()
    .include(vec!["user.slice/*".to_string()])
    .build()
    .expect("Failed to build explorer");
let found = explorer
    .iter_cgroups()
    .for_each(|c| println!("Found cgroup: {}", c.path()));
```
