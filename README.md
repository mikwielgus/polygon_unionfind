# polygon_unionfind

[![Repository](https://img.shields.io/badge/repository-GitHub-0FBF3E)](https://github.com/mikwielgus/multi_bimap)
[![Docs](https://docs.rs/undoredo/badge.svg)](https://docs.rs/polygon_unionfind/)
[![Crates.io](https://img.shields.io/crates/v/polygon_unionfind.svg)](https://crates.io/crates/polygon_unionfind)
[![MIT OR Apache 2.0](https://img.shields.io/crates/l/polygon_unionfind.svg)](#licence)

`polygon_unionfind` is a Rust library that implements disjoint-polygon data
structure (polygon union-find): a disjoint-set data structure where sets are
polygons.

Upon insertion, each polygon is merged with all the intersecting polygons that
are already present. To reduce the cost of finding intersecting polygons, they
are spatially filtered with an R-tree.

This library has optional integration with
[`undoredo`](https://crates.io/crates/undoredo), a crate which provides
[Undo/Redo](https://en.wikipedia.org/wiki/Undo) functionality for any data
structure, allowing to programmatically revert and restore any change using
deltas or snapshots.

## Demo

![Animation showing polygons being added and subtracted in the demo stored in
`demos/polygon_set/` directory](polygon_set_demo.gif)

For an interactive visualization of `polygon_unionfind`'s polygon addition,
subtraction, and undo-redo action, you can run our graphical demos implemented
using the cross-platform [`macroquad`](https://crates.io/crates/macroquad)
crate by running the following commands. The first one is shown in action on the
animation above.

```sh
cargo run --manifest-path=demos/polygon_set/Cargo.toml
```

```sh
cargo run --manifest-path=demos/combinators/Cargo.toml
```

```sh
cargo run --manifest-path=demos/layers_with_transitions/Cargo.toml
```

## Usage

### Adding dependency

First, add `polygon_unionfind` as a dependency to your `Cargo.toml`:

```toml
[dependencies]
polygon_unionfind = { version = "0.10.0", features = ["undoredo"] }
```

If you don't need to perform undo and redo operations, you can remove the
`undoredo` feature.

### Basic usage

Following is a basic usage example of `polygon_unionfind`.

```rust
use polygon_unionfind::{PolygonUnionFind, PolygonWithData};

let mut polygon_unionfind = PolygonUnionFind::<i32, PolygonWithData<i32, &str>>::new();

let polygon1 = polygon_unionfind.add(PolygonWithData {
    exterior: vec![
        [0, 0],
        [3, 0],
        [4, 2],
        [3, 4],
        [0, 4],
        [-1, 2],
    ],
    interiors: vec![],
    data: "first",
});

let polygon2 = polygon_unionfind.add(PolygonWithData {
    exterior: vec![
        [2, 0],
        [5, 0],
        [6, 2],
        [5, 4],
        [2, 4],
        [1, 2],
    ],
    interiors: vec![],
    data: "second",
});

// Overlapping polygons are now in the same set and thus have the same
// representative.
let polygon1_repr = polygon_unionfind.find(polygon1.index()).exterior.len();
let polygon2_repr = polygon_unionfind.find(polygon2.index()).exterior.len();
assert_eq!(polygon1_repr, polygon2_repr);
```

## Documentation

See the
[documentation](https://docs.rs/polygon_unionfind/latest/polygon_unionfind) for
more information about `polygon_unionfind`'s usage.

## Packaging

`polygon_unionfind` is published as a
[crate](https://crates.io/crates/polygon_unionfind) on the
[Crates.io](https://crates.io/) registry.

## Contributing

We welcome issues and pull requests from anyone to our
[repository](https://github.com/mikwielgus/polygon_unionfind) on GitHub.

## Licence

### Outbound licence

`polygon_unionfind` is dual-licensed as under either of

- [MIT license](./LICENSES/MIT.txt),
- [Apache License, Version 2.0](./LICENSES/Apache-2.0.txt),

at your option.

### Inbound licence

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you will be dual-licensed as described above,
without any additional terms or conditions.
