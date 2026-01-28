---
title: plane-cli base url env
status: closed
priority: 2
issue-type: task
created-at: "\"\\\"2026-01-28T15:40:34.450282+07:00\\\"\""
closed-at: "2026-01-28T15:41:01.086101+07:00"
close-reason: "Added PLANE_BASE_URL parsing + docs; bumped to 0.1.1. Validation: cargo build --release; plane user me-list; plane project list"
---

Add PLANE_BASE_URL support; adjust README; verify with live API. Files: src/main.rs, README.md, Cargo.toml, Formula/plane-cli.rb. AC: base url works; user/project calls succeed.
