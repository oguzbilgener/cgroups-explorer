use std::path::{Path, PathBuf};

use cgroups_rs::{
    Cgroup, Hierarchy,
    hierarchies::{V1, V2, is_cgroup2_unified_mode},
};
use derive_builder::Builder;
use walkdir::WalkDir;

/// An interface to explore cgroups in the system.
///
/// # Example
/// ```rust
/// use cgroups_explorer::Explorer;
/// let explorer = Explorer::detect_version()
///     .include(vec!["user.slice/*".to_string()])
///     .build()
///     .expect("Failed to build explorer");
/// let found = explorer
///     .iter_cgroups()
///     .for_each(|c| println!("Found cgroup: {}", c.path()));
///
/// ```
#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Explorer {
    /// The cgroup hierarchy to explore.
    hierarchy: Box<dyn Hierarchy>,

    /// The globs to include in the exploration.
    #[builder(field(ty = "Vec<String>", build = "parse_include(self.include)?"))]
    include: Vec<glob::Pattern>,
}

/// An iterator over cgroups in the system that match the globs.
pub struct CgroupsIterator {
    walker: walkdir::IntoIter,
    include: Vec<glob::Pattern>,
    hierarchy: Box<dyn Hierarchy>,
    base_path: PathBuf,
}

impl Explorer {
    /// Create a new `ExplorerBuilder` for cgroups v1.
    #[must_use]
    pub fn v1() -> ExplorerBuilder {
        ExplorerBuilder::default().hierarchy(Box::new(V1::new()))
    }

    /// Create a new `ExplorerBuilder` for cgroups v2.
    #[must_use]
    pub fn v2() -> ExplorerBuilder {
        ExplorerBuilder::default().hierarchy(Box::new(V2::new()))
    }

    /// Create a new `ExplorerBuilder` by detecting the cgroups version on the system.
    #[must_use]
    pub fn detect_version() -> ExplorerBuilder {
        if is_cgroup2_unified_mode() {
            ExplorerBuilder::default().hierarchy(Box::new(V2::new()))
        } else {
            ExplorerBuilder::default().hierarchy(Box::new(V1::new()))
        }
    }

    fn hierarchy(&self) -> Box<dyn Hierarchy> {
        if self.hierarchy.v2() {
            Box::new(V2::new())
        } else {
            Box::new(V1::new())
        }
    }

    /// Create an iterator over all cgroups in the system, based on the criteria.
    #[must_use]
    pub fn iter_cgroups(&self) -> CgroupsIterator {
        let base_path = self.hierarchy.root();
        let walker = WalkDir::new(base_path.clone())
            .min_depth(1)
            .sort_by_file_name()
            .into_iter();
        CgroupsIterator {
            walker,
            include: self.include.clone(),
            hierarchy: self.hierarchy(),
            base_path,
        }
    }
}

impl Iterator for CgroupsIterator {
    type Item = Cgroup;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.walker.next();
            match entry {
                Some(Ok(entry)) => {
                    let path = entry.path();
                    if !entry.file_type().is_dir() {
                        continue;
                    }
                    let Ok(relative_path) = path.strip_prefix(&self.base_path) else {
                        continue;
                    };
                    if relative_path.components().count() == 0 {
                        continue;
                    }
                    if !self.matches_include(relative_path) {
                        continue;
                    }
                    return Some(Cgroup::load(self.hierarchy(), path));
                }
                Some(Err(_e)) => return None,
                None => return None,
            }
        }
    }
}

impl CgroupsIterator {
    fn matches_include(&self, path: &Path) -> bool {
        if self.include.is_empty() {
            return true;
        }
        let path_str = path.to_string_lossy();
        self.include
            .iter()
            .any(|pattern| pattern.matches(&path_str))
    }

    fn hierarchy(&self) -> Box<dyn Hierarchy> {
        if self.hierarchy.v2() {
            Box::new(V2::new())
        } else {
            Box::new(V1::new())
        }
    }
}

fn parse_include(include: Vec<String>) -> Result<Vec<glob::Pattern>, ExplorerBuilderError> {
    if include.is_empty() {
        Ok(Vec::new())
    } else {
        include
            .into_iter()
            .map(|include| {
                glob::Pattern::new(&include)
                    .map_err(|e| ExplorerBuilderError::ValidationError(e.to_string()))
            })
            .collect()
    }
}
