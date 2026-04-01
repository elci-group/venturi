# Venturi

Venturi is a Rust-based, DAG-native programming language and runtime environment. It is designed to model execution and organization as Directed Acyclic Graphs (DAGs), ensuring that data flows predictably without infinite loops. With strong typing, built-in data frames, explicit data-flow wiring, and built-in permissions/versioning, Venturi is the ideal engine for data engineering pipelines, workflow automation, and distributed UI components.

## Features

- **DAG-Native Execution**: Wires components explicitly (`A -> B`), ensuring cyclic dependencies are impossible.
- **Built-in Types**: First-class support for `Int`, `Float`, `Bool`, `String`, and `DataFrame`.
- **Modularity & Composition**: Import and use distributed components with `use chassis as` and fetch from remote sources securely using `pit @url`.
- **Secure Sandboxing**: Strict permissions and versioning controlled by the `VAN:` schema syntax.
- **Graphical Interfaces**: Integrated support for windowed applications (via `egui` and `iced` backends) where UI components are nodes in a reactive data-flow graph.

## Installation

Ensure you have [Rust and Cargo](https://rustup.rs/) installed.

```bash
git clone git@github.com:elci-group/venturi.git
cd venturi
cargo build --release
```

The compiled binary will be available at `target/release/venturi`.

## Quick Start

### Running a single file

```bash
venturi run my_script.vt
```

### Running a DAG directory

```bash
venturi run-dag my_pipeline_dir/
```

### Launching a GUI app

```bash
venturi gui --dir gui-app/ --backend egui
```

## Documentation

- `man venturi`: Detailed command-line reference (available in `man/venturi.1`).
- `AGENTS.md`: Codebase architecture and contributor guidelines.
- `CONTRIBUTING.md`: Contribution guidelines.

## License

MIT License
