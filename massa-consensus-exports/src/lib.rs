// Copyright (c) 2022 MASSA LABS <info@massa.net>
//! Consensus exports
#![feature(async_closure)]
#![feature(bool_to_option)]
#![feature(hash_drain_filter)]
#![feature(map_first_last)]
#![feature(int_roundings)]
#![warn(missing_docs)]
#![warn(unused_crate_dependencies)]
#[macro_use]
extern crate massa_logging;

pub use consensus_controller::{ConsensusCommandSender, ConsensusEventReceiver, ConsensusManager};
pub use error::ConsensusError;
use massa_models::{Address, Slot};
pub use settings::{ConsensusConfig, ConsensusSettings};

mod consensus_controller;

/// consensus errors
pub mod error;

/// consensus settings
pub mod settings;

/// consensus commands
pub mod commands;

/// consensus events
pub mod events;

/// For a slot associate the selected node's addresses for draws by a node address
type SelectionDraws = Vec<(Slot, (Address, Vec<Address>))>;

/// consensus test tools
pub mod tools;
