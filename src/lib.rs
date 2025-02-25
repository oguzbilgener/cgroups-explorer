//! # cgroups-explorer

#[doc = include_str!("../README.md")]
mod explorer;

pub use explorer::{CgroupsIterator, Explorer, ExplorerBuilder, ExplorerBuilderError};
#[cfg(test)]
mod tests;
