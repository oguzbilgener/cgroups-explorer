//! # cgroups-explorer

#![allow(clippy::multiple_crate_versions)]

#[doc = include_str!("../README.md")]
mod explorer;

pub use explorer::{Explorer, ExplorerBuilder, ExplorerBuilderError};
#[cfg(test)]
mod tests;
