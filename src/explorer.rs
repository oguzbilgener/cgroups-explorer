use std::{
    collections::{HashSet, hash_set},
    path::{Path, PathBuf},
};

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
    /// The regexes to match group names against.
    #[cfg_attr(
        feature = "regex",
        builder(field(ty = "Vec<String>", build = "parse_include_regex(self.include_regex)?"))
    )]
    #[cfg(feature = "regex")]
    include_regex: Vec<regex::Regex>,
}

/// An iterator over cgroups in the system that match the globs.
struct CgroupsV2Iterator {
    walker: walkdir::IntoIter,
    include: Vec<glob::Pattern>,
    #[cfg(feature = "regex")]
    include_regex: Vec<regex::Regex>,
    base_path: PathBuf,
}

struct CgroupsV1Iterator {
    discovered: hash_set::IntoIter<PathBuf>,
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

    /// Create an iterator over all cgroups in the system, based on the criteria.
    #[must_use]
    pub fn iter_cgroups(&self) -> Box<dyn Iterator<Item = Cgroup>> {
        if self.hierarchy.v2() {
            Box::new(self.iter_cgroups_v2())
        } else {
            Box::new(self.iter_cgroups_v1())
        }
    }

    fn iter_cgroups_v2(&self) -> CgroupsV2Iterator {
        let base_path = self.hierarchy.root();
        let walker = WalkDir::new(base_path.clone())
            .min_depth(1)
            .sort_by_file_name()
            .into_iter();
        CgroupsV2Iterator {
            walker,
            include: self.include.clone(),
            #[cfg(feature = "regex")]
            include_regex: self.include_regex.clone(),
            base_path,
        }
    }

    fn iter_cgroups_v1(&self) -> CgroupsV1Iterator {
        let hierarchy = V1::new();
        let subystems = hierarchy.subsystems();
        let base_path = hierarchy.root();

        let mut matching_rel_paths = HashSet::new();
        for subsystem in subystems {
            let name = subsystem.controller_name();
            let walker = WalkDir::new(base_path.join(&name))
                .min_depth(1)
                .sort_by_file_name()
                .into_iter();
            let base_controller_path = base_path.join(name);
            for entry in walker {
                let Ok(entry) = entry else { continue };
                let path = entry.path();
                if !entry.file_type().is_dir() {
                    continue;
                }
                let Ok(relative_path) = path.strip_prefix(&base_controller_path) else {
                    continue;
                };
                if relative_path.components().count() == 0 {
                    continue;
                }
                #[cfg(feature = "regex")]
                let should_include = path_matches_include(&self.include, relative_path)
                    || path_matches_include_regex(&self.include_regex, relative_path);
                #[cfg(not(feature = "regex"))]
                let should_include = path_matches_include(&self.include, relative_path);

                if should_include {
                    matching_rel_paths.insert(relative_path.to_path_buf());
                }
            }
        }

        CgroupsV1Iterator {
            discovered: matching_rel_paths.into_iter(),
        }
    }
}

impl Iterator for CgroupsV2Iterator {
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
                    if !path_matches_include(&self.include, relative_path) {
                        continue;
                    }
                    #[cfg(feature = "regex")]
                    if !path_matches_include_regex(&self.include_regex, relative_path) {
                        continue;
                    }
                    return Some(Cgroup::load(Box::new(V2::new()), relative_path));
                }
                Some(Err(_e)) => return None,
                None => return None,
            }
        }
    }
}

impl Iterator for CgroupsV1Iterator {
    type Item = Cgroup;

    fn next(&mut self) -> Option<Self::Item> {
        self.discovered
            .next()
            .map(|path| Cgroup::load(Box::new(V1::new()), path))
    }
}

fn path_matches_include(include: &[glob::Pattern], path: &Path) -> bool {
    if include.is_empty() {
        return true;
    }
    let path_str = path.to_string_lossy();
    include.iter().any(|pattern| pattern.matches(&path_str))
}

#[cfg(feature = "regex")]
fn path_matches_include_regex(include: &[regex::Regex], path: &Path) -> bool {
    if include.is_empty() {
        return true;
    }
    let path_str = path.to_string_lossy();
    include.iter().any(|pattern| pattern.is_match(&path_str))
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

#[cfg(feature = "regex")]
fn parse_include_regex(include: Vec<String>) -> Result<Vec<regex::Regex>, ExplorerBuilderError> {
    if include.is_empty() {
        Ok(Vec::new())
    } else {
        include
            .into_iter()
            .map(|include| {
                regex::Regex::new(&include)
                    .map_err(|e| ExplorerBuilderError::ValidationError(e.to_string()))
            })
            .collect()
    }
}
