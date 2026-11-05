# Standard library compatibility

The former Rust core workaround patch is retired and must not be applied. Its
historical contents remain in Git history; it is not an active source dependency.
Do not treat the earlier migration receipt as evidence of today's checkout state.

Before claiming upstream standard-library compatibility, verify source-based
constant evaluation, signed integer intrinsics, zero-shift rotations, integer/byte
array transmutation, slice copy and drop-glue availability through the compiler.
Use the current compiler and frontend implementations; do not restore bypasses or
nonportable native-endian rewrites to make acceptance pass. This document records
remaining verification scope, not a claim that these cases currently succeed.
