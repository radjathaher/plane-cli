# plane-cli

Auto-generated Plane CLI from Plane REST API URL patterns. Designed for LLM discovery and direct scripting.

## Install

### Install script (macOS arm64 + Linux x86_64)

```bash
curl -fsSL https://raw.githubusercontent.com/radjathaher/plane-cli/main/scripts/install.sh | bash
```

### Homebrew (binary, macOS arm64 only)

```bash
brew tap radjathaher/tap
brew install plane-cli
```

### Build from source

```bash
cargo build --release
./target/release/plane --help
```

## Auth

Get a Plane API key and set it in your shell:

```bash
export PLANE_API_KEY="..."
```

Optional overrides:

```bash
export PLANE_API_URL="https://api.plane.so"
export PLANE_API_BASE_PATH="/api/v1"
export PLANE_WORKSPACE="my-workspace"
```

## Discovery (LLM-friendly)

```bash
plane list --json
plane describe work-item list --json
plane tree --json
```

Human help:

```bash
plane --help
plane project --help
plane project list --help
```

## Examples

List projects:

```bash
plane project list --slug my-workspace --pretty
```

Create work item:

```bash
plane work-item create \
  --slug my-workspace \
  --project-id <PROJECT_ID> \
  --body-json '{"name":"Fix login", "priority":"high"}' \
  --pretty
```

Raw request:

```bash
plane request GET /api/v1/users/me/ --pretty
```

## Update command tree

```bash
PLANE_REPO_PATH=/path/to/plane tools/gen_command_tree.py --plane-repo "$PLANE_REPO_PATH" --out schemas/command_tree.json
cargo build
```

## Notes

- `--fields` and `--expand` map to Plane API query parameters.
- Deprecated `/issues` endpoints are hidden; use `--include-deprecated` to access.
