# Mallang Binary Archive

This archive contains the public `mlg` driver and its sibling self-hosted
compiler core, `mlgc`, for the target named in the archive filename. Keep both
binaries in the same directory. Run `bin/mlg --version --verbose` and
`bin/mlg --help` to inspect the installed compiler.

`mlg` uses the Mallang-built `mlgc` core by default. The Rust Stage0 seed remains
embedded in the driver as an explicit offline recovery path:
`bin/mlg --compiler stage0 <subcommand> ...`.

Mallang lowers programs to C. `mlg build`, `mlg run`, and `mlg test` require
`clang` to be available on `PATH`; install the platform C compiler tools before
using those commands.

The binary is distributed under your choice of the MIT License or the Apache
License, Version 2.0. See `LICENSE-MIT` and `LICENSE-APACHE` in this archive.
