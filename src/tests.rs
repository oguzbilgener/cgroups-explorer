use cgroups_rs::{
    Cgroup, cgroup_builder::CgroupBuilder, cpu::CpuController, hierarchies::auto,
    memory::MemController,
};
use serial_test::serial;

use crate::explorer::Explorer;

#[test]
#[serial]
fn explore_created_cgroups() -> anyhow::Result<()> {
    let h = cgroups_rs::hierarchies::auto();

    let cgroup_name = "test_cgroup_explorer";
    let existing_cgroup = Cgroup::load(auto(), auto().root().join(cgroup_name));
    let _ = existing_cgroup.delete();
    let cgroup: Cgroup = CgroupBuilder::new(cgroup_name)
        .memory()
        .memory_swap_limit(3 * 1024)
        .memory_soft_limit(512 * 1024)
        .memory_hard_limit(1024 * 1024)
        .done()
        .cpu()
        .shares(90)
        .done()
        .build(h)
        .unwrap();

    let explorer = Explorer::detect_version().build()?;
    let found = explorer
        .iter_cgroups()
        .find(|c| c.path().ends_with(&cgroup_name))
        .expect("cgroup not found");

    assert!(found.exists());
    let cpu: &CpuController = found.controller_of().expect("No cpu controller attached");
    let memory: &MemController = found
        .controller_of()
        .expect("No memory controller attached");
    assert_eq!(memory.memory_stat().soft_limit_in_bytes, 512 * 1024);
    assert_eq!(memory.memory_stat().limit_in_bytes, 1024 * 1024);
    assert_eq!(cpu.shares()?, 90);

    cgroup.delete()?;

    Ok(())
}

#[test]
#[serial]
#[cfg(feature = "regex")]
fn explore_created_cgroups_regex() -> anyhow::Result<()> {
    let h = cgroups_rs::hierarchies::auto();

    let cgroup_name = "test_cgroup_explorer2";
    let existing_cgroup = Cgroup::load(auto(), auto().root().join(cgroup_name));
    let _ = existing_cgroup.delete();
    let cgroup: Cgroup = CgroupBuilder::new(cgroup_name)
        .memory()
        .memory_swap_limit(3 * 1024)
        .memory_soft_limit(512 * 1024)
        .memory_hard_limit(1024 * 1024)
        .done()
        .cpu()
        .shares(90)
        .done()
        .build(h)
        .unwrap();

    let explorer = Explorer::detect_version()
        .include_regex(vec!["^test_.*?_explorer[0-9]$".to_string()])
        .build()?;
    let found = explorer
        .iter_cgroups()
        .find(|c| c.path().ends_with(&cgroup_name))
        .expect("cgroup not found");

    assert!(found.exists());
    let cpu: &CpuController = found.controller_of().expect("No cpu controller attached");
    let memory: &MemController = found
        .controller_of()
        .expect("No memory controller attached");
    assert_eq!(memory.memory_stat().soft_limit_in_bytes, 512 * 1024);
    assert_eq!(memory.memory_stat().limit_in_bytes, 1024 * 1024);
    assert_eq!(cpu.shares()?, 90);

    cgroup.delete()?;

    Ok(())
}
