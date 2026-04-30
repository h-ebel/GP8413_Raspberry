# GP8413_Raspberry
Writing to the DAC-chip GP8413 (DFR1073) directly from C++ or Rust using basic write commands instead of some library - tested on a Raspberry Pi 500+.

The Rust code is the author's very first compile-ready Rust example, so be kind. 
It was at least checked with `cargo fmt --check` (no output left) and `cargo clippy` (only two warnings left that sign of a value might be lost and that a value might be truncated by an explicit cast - but should be logically fine through previous checks; has to do with minimal and maximal voltage value)
