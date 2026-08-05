//! Pluggable tool adapters (git, search, browser, workspace, …) and [`ToolManager`].
//!
//! Not to be confused with [`crate::core::runtime`] (chat protocol types:
//! `ChatMessage`, `StreamEvent`) or [`crate::core::agent::runtime`] (agent run
//! lifecycle). See `docs/rust-architecture.md`.

pub mod ai;
pub mod browser;
pub mod config;
pub mod context;
pub mod conversation;
pub mod event;
pub mod file;
pub mod git;
pub mod memory;
pub mod search;
pub mod settings;
pub mod terminal;
pub mod tool;
pub mod workspace;

pub use tool::ToolManager;
