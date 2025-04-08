//! Rust Documentation MCP Service
//! 
//! This crate provides a Machine Control Protocol (MCP) service for fetching and caching
//! Rust documentation from docs.rs. It allows clients to request documentation for any
//! published Rust crate through a simple interface.
//!
//! # Features
//!
//! - Fetch documentation from docs.rs
//! - Cache documentation for faster subsequent access
//! - MCP server implementation for remote access
//! - Support for versioned documentation
//!
//! # Modules
//!
//! - [`cache`]: Caching implementation for documentation
//! - [`docs_parser`]: Interface with docs.rs and documentation parsing
//! - [`mcp`]: MCP server implementation and protocol handling

pub mod cache;
pub mod docs_parser;
pub mod mcp;