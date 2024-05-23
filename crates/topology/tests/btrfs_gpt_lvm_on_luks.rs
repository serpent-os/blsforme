// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Testing BTRFS volume on LVk atop LUKS...

use std::{env, path::PathBuf};

use topology::disk::builder;

#[test]
fn topology_test() {
    let topo = builder::new()
        .with_devfs(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/btrfs_gpt_lvm_on_luks/dev"
        ))
        .with_sysfs(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/btrfs_gpt_lvm_on_luks/sys"
        ))
        .with_procfs(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/btrfs_gpt_lvm_on_luks/proc"
        ))
        .build()
        .expect("Failed to create Probe");

    let root_device = topo
        .get_device_from_mountpoint("/")
        .expect("Cannot find root device");
    assert_eq!(
        root_device,
        PathBuf::from("tests/btrfs_gpt_lvm_on_luks/dev/mapper/BogusInstall-root")
    );
    let sb = topo.get_device_superblock(root_device).expect("need uuid");
    assert_eq!(sb.uuid().unwrap(), "2a78a4da-f110-4441-8839-dbd97ab87cda");
    assert_eq!(sb.kind(), superblock::Kind::Btrfs);
    let block = topo
        .get_rootfs_device("/")
        .expect("Failed to determine block device");

    let cmdline = block.cmd_line();
    // PartUUID is the only one we want.
    assert_eq!(
        cmdline,
        "rd.luks.uuid=b6b31f26-39f4-48f7-bed5-6faaff96cca4 root=UUID=2a78a4da-f110-4441-8839-dbd97ab87cda rootfsflags=subvol=/"
    );
}
