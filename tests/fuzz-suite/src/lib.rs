//! # UMCF Fuzz Suite — Protocol Parser & FFI Fuzzing Harness
//!
//! Provides coverage-guided LibFuzzer targets, bounded slice readers,
//! pre-allocation length caps, and minimized seed corpus regressions.

#![forbid(unsafe_code)]

pub mod bounded_reader;
pub mod corpus_runner;
pub mod error;
pub mod targets;

pub use bounded_reader::{
    check_nesting_depth, BoundedSliceReader, MAX_ALLOCATION_BYTES, MAX_RECURSION_DEPTH,
};
pub use corpus_runner::{CorpusRunSummary, CorpusRunner};
pub use error::FuzzError;
pub use targets::FuzzTargets;
