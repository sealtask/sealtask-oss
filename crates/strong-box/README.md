# SealTask StrongBox fork

This is SealTask's GPL-3.0-only fork of
[`mpalmer/strong-box`](https://github.com/mpalmer/strong-box). It carries the
WASM entropy hook required by the adjacent `strong-box-wasm` browser bindings.
The canonical SealTask source is published inside
[`sealtask/sealtask-oss`](https://github.com/sealtask/sealtask-oss), and the
production browser artifact is built directly from this directory.

This crate provides safe, ergonomic encryption "boxes" for storing data you'd prefer not to have exposed to the world.
It uses modern, fast cryptography in misuse-resistant ways.

Its spiritual ancestor is NaCl / libsodium, and uses many of the same cryptographic primitives, but is not at all compatible with them.

See [the docs](https://docs.rs/strong-box) for all the gory details.

# MSRV

Specified in `Cargo.toml`.
Bumping is a breaking change.

# Licence

Unless otherwise stated, everything in this repo is covered by the following
copyright notice:

```text
    Copyright (C) 2024  Matt Palmer <matt@hezmatt.org>

    This program is free software: you can redistribute it and/or modify it
    under the terms of the GNU General Public License version 3, as
    published by the Free Software Foundation.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
```
