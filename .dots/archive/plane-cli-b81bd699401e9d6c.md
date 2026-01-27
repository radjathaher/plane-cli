---
title: plane-cli initial
status: closed
priority: 1
issue-type: task
created-at: "\"\\\"2026-01-27T21:44:13.074683+07:00\\\"\""
closed-at: "2026-01-27T21:59:44.164637+07:00"
close-reason: "Built Rust Plane CLI with generated command tree, request escape hatch, install + formula, release asset; repo + tag v0.1.0. Validation: cargo build --release"
---

Build Plane CLI (Rust) mirroring linear-cli. Scope: command tree from Plane API URLs, env auth, request exec, install script + Homebrew formula + release artifact. Files: Cargo.toml, src/**, tools/**, schemas/**, scripts/**, Formula/**, README.md. AC: CLI runs, env auth, list/describe/tree, request path ops, arm64 macOS binary + release asset.
