# GP8413_Raspberry
Writing to the DAC-chip GP8413 (DFR1073) directly from C++ or Rust using basic write commands instead of some library - tested on a Raspberry Pi 500+. It is purpusefully not a directly useful program as it outputs a voltage fixed at compile time, as the example shall rather introduce the necessary commands and library calls in a minimal manner, inspiring to include the DAC into more useful workflows. 

The Rust code is the author's very first compile-ready Rust example, so be kind. 
It was at least checked with `cargo fmt --check` (no output left) and `cargo clippy` (only two warnings left that sign of a value might be lost and that a value might be truncated by an explicit cast - but should be logically fine through previous checks; has to do with minimal and maximal voltage value).
