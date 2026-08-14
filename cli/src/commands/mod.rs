//! One module per CLI subcommand wrapper, mirroring `living_docs_core::commands` (issue 0028).

pub(crate) mod brief;
pub(crate) mod check;
pub(crate) mod db;
pub(crate) mod describe;
pub(crate) mod export;
pub(crate) mod fmt;
pub(crate) mod hooks_cmd;
pub(crate) mod index;
pub(crate) mod leak_gate;
pub(crate) mod migrate;
pub(crate) mod new;
pub(crate) mod next;
pub(crate) mod seal_cmd;
pub(crate) mod search;
pub(crate) mod skill_cmd;
pub(crate) mod status;
pub(crate) mod supersede;
