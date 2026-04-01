#!/usr/bin/env bash
# run_benchmarks.sh

echo "========================================"
echo "      Venturi Benchmark Suite           "
echo "========================================"

echo "1. Building Venturi (release mode)..."
cargo build --release

echo ""
echo "2. Running Python benchmark (100,000 iterations)..."
time python3 benches/arithmetic.py

echo ""
echo "3. Running Node.js benchmark (100,000 iterations)..."
time node benches/arithmetic.js

echo ""
echo "4. Running Venturi benchmark (100,000 iterations)..."
# Note: Since Venturi's CLI doesn't currently support looping executions 
# internally from the CLI, we benchmark the DAG startup and parsing time
# to show where Venturi's overhead lies compared to scripting engines.
time target/release/venturi run benches/arithmetic.vt > /dev/null

echo ""
echo "========================================"
echo "Note: Full continuous throughput benchmarks require"
echo "the Venturi Runtime to be embedded in a Rust harness."
