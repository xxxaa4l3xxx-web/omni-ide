# Omni IDE

A next-generation, multi-language integrated development environment written in Rust.

## Architecture

| Crate         | Responsibility                                  |
|---------------|-------------------------------------------------|
| `omni-core`   | Rope-based text buffers, tabs, file manager     |
| `omni-syntax` | Tree-sitter syntax highlighting (universal)      |
| `omni-lsp`    | Language Server Protocol client (tower-lsp)      |
| `omni-app`    | GPU-accelerated GUI (eframe / egui)              |

## Design Goals

- **Universal language support** — Tree-sitter grammars + LSP registry.
- **Extreme performance** — Rope data structure for O(log n) edits on multi-GB files.
- **Memory safety** — Rust guarantees zero data races or use-after-free.
- **Lightweight** — Sub-100 MB idle, native binary, no Electron bloat.
- **Plugin ready** — WASM plugin system (future milestone).

## Building

```bash
cargo build --release
```

## License

MIT OR Apache-2.0
