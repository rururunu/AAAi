//! Agent run orchestration shell — session/run lifecycle around the real loop.
//!
//! **Read this first:** the model↔tools conversation loop lives in
//! [`crate::core::chat::agent::AgentRunner`] and [`crate::core::chat::agent_loop`].
//! This module owns run state, event bridging, cancel/soft-inject, and debug
//! snapshots; [`runtime::AgentRuntime::spawn`] delegates streaming work to
//! [`crate::core::chat::stream::StreamManager`] → `AgentRunner`.
//!
//! Planner / executor here are for run-level plan steps and a tool facade —
//! not a second agent loop. See `docs/rust-architecture.md`.

#![allow(dead_code, unused_imports)] // plan-step helpers not on every hot path

pub mod debug;
pub mod executor;
pub mod planner;
pub mod runtime;
pub mod tools;

pub use debug::AgentDebugEvent;
pub use runtime::{
    AgentEvent, AgentEventRecord, AgentRun, AgentRuntime, AgentSpawnInput, AgentState,
};
