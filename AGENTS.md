# Venturi - Agents Guide

Welcome to the Venturi codebase! This document provides the necessary context, commands, and patterns to work effectively within this repository.

## Project Overview

Venturi is a Rust-based project that parses, lexes, and manages a custom domain-specific language (DSL). The language relies on Directed Acyclic Graph (DAG) structures for execution and organization. It features custom node types, typed inputs/outputs, and data-flow wiring.

## Essential Commands

As a standard Rust project, use the Cargo toolchain:

- **Build:** `cargo build`
- **Run:** `cargo run`
- **Test:** `cargo test`
- **Lint:** `cargo clippy`
- **Format:** `cargo fmt`

## Code Organization & Structure

The codebase is organized under `src/`:

- `main.rs`: The entry point of the Venturi application.
- `error.rs`: Centralized error handling using `thiserror`. Defines `VenturiError` and the custom `Result` type.
- `lexer/mod.rs`: A custom lexer that tokenizes the DSL. Handles indentation, shebangs, custom keywords (`input`, `output`, `func`, `pit`, `chassis`, etc.), and special patterns (`VAN:`, `META:`).
- `parser/mod.rs`: Consumes tokens to produce the Abstract Syntax Tree (AST). Parses inputs, outputs, DAG wires (`A -> B`), functions, and `try/catch` blocks.
- `ast/mod.rs`: Defines the AST structures (`VtFile`, `NodeKind`, `InputDecl`, `Stmt`, `Expr`, etc.).
- `graph/mod.rs`: Implements the DAG using `petgraph`. Manages nodes (`Module`, `Chass`, `Pit`), edges, topological sorting, and cycle detection.

## Core DSL Concepts

When working on the parser or ast, be aware of the language features:

- **Node Kinds:** Files typically represent a `Plane` or `Vortex`, often denoted by a shebang (e.g., `#! plane`).
- **Data Flow (DAG Wires):** Represented by arrows (e.g., `ident -> ident`).
- **Declarations:** Uses `input`, `output`, `use chassis as`, and `pit @url`.
- **Types:** Built-in types include `Int`, `Float`, `Bool`, `String`, and `DataFrame`, plus custom identifiers.
- **Control Flow:** Supports `func` declarations, and `try` / `catch` blocks.
- **Metadata & Security:** Uses `VAN:` for permissions/versioning and `META:` or `key: value` comments for metadata.

## Naming Conventions & Style Patterns

- **Error Handling:** Always use `crate::error::Result` and `crate::error::VenturiError` for fallible operations.
- **AST Node Naming:** Uses PascalCase for structs and enums (e.g., `VtFile`, `InputDecl`, `UseChass`, `FuncDef`).
- **Lexer/Parser:** The lexer generates `SpannedToken` containing the token, line, and column. The parser consumes these to construct the `VtFile` AST. Indentation is significant in the DSL and is tracked via `Indent` and `Dedent` tokens.

## Important Gotchas

- **Whitespace/Indentation Sensitivity:** The custom DSL uses significant whitespace (indentation-based blocks for functions and try/catch). The Lexer emits `Indent` and `Dedent` tokens similar to Python.
- **Cycle Detection:** The DAG enforces strict acyclic constraints. Cycle detection (`is_cyclic_directed`) runs immediately when adding edges in `graph/mod.rs`.
- **Custom Result Tokens:** The language includes special compound tokens for `Result.Ok` and `Result.Err` built into the lexer and parser.
