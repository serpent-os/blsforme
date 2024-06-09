// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Ensure proper detection of ext4+gpt roots

use std::{env, path::PathBuf};

use topology::disk::builder;

#[test]
fn topology_test() {
    let topo = builder::new()
        .with_devfs(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ext4_gpt/dev"))
        .with_sysfs(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ext4_gpt/sys"))
        .with_procfs(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ext4_gpt/proc"))
        .build()
        .expect("Failed to create Probe");

    let root_device = topo.get_device_from_mountpoint("/").expect("Cannot find root device");
    assert_eq!(root_device, PathBuf::from("tests/ext4_gpt/dev/nvme0n1p1"));
    let sb = topo.get_device_superblock(root_device).expect("need uuid");
    assert_eq!(sb.uuid().unwrap(), "1f5cb158-4a0e-48e2-a339-157d8133f05f");
    assert_eq!(sb.kind(), superblock::Kind::Ext4);
    let block = topo.get_rootfs_device("/").expect("Failed to determine block device");

    let cmdline = block.cmd_line();
    // PartUUID is the only one we want.
    assert_eq!(cmdline, "root=PARTUUID=6ca59a0c-e8c9-4ec4-b331-351d120fbb32");
}
