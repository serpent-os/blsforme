// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Disk probe/query APIs

use thiserror::Error;

pub mod builder;
pub mod mounts;
pub mod probe;

#[derive(Debug, Error)]
pub enum Error {
    #[error("from io: {0}")]
    IO(#[from] std::io::Error),
}
