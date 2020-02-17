# scoped-vec.rs

[![License: WTFPL](https://img.shields.io/badge/License-WTFPL-brightgreen.svg?style=flat-square&logo=appveyor)](http://www.wtfpl.net/about/) ![https://docs.rs/scoped-vec/](https://docs.rs/scoped-vec/badge.svg) [![Downloads](https://img.shields.io/crates/d/scoped-vec.svg?style=flat-square&logo=appveyor)](https://crates.io/crates/scoped-vec)

A library for scoped `Vec`s, allowing multi-level divergence from the root element.

This is useful for monitoring state within a de facto tree where
links to parents aren't necessarily needed. Consumers can keep
references to a specific parent if required and check the values
from the scope of their choosing, parents are free to be dropped if
they're no longer required.


The full `std::vec::Vec` spec has not yet been implemented but as
the library stabilises, more and more of the `Vec` library will be
supported - however there will be some divergence from the API where
necessary given the structural differences of a `ScopedVec`.

The library isn't yet ready for consumption in any production-level
software but feel free to use it in side projects and make contributions
where you find necessary.