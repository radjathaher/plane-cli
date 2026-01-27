#!/usr/bin/env python3
import argparse
import json
import os
import re
from dataclasses import dataclass
from typing import Dict, Iterable, List, Optional, Tuple

PATH_CALL_RE = re.compile(r"path\(\s*\"([^\"]+)\"", re.MULTILINE)
METHODS_RE = re.compile(r"http_method_names\s*=\s*\[([^\]]+)\]", re.MULTILINE)
METHOD_TOKEN_RE = re.compile(r"\"([a-z]+)\"")
PARAM_RE = re.compile(r"<[^>]+>")

IGNORE_PREFIXES = {"workspaces", "projects"}

RESOURCE_SEGMENTS = {
    "project": ["projects"],
    "cycle": ["cycles"],
    "module": ["modules"],
    "state": ["states"],
    "label": ["labels"],
    "member": ["members", "project-members"],
    "intake": ["intake-issues"],
    "user": ["users"],
    "asset": ["assets", "user-assets"],
    "work-item": ["work-items"],
    "issue": ["issues"],
    "invite": ["invitations"],
    "sticky": ["stickies"],
}

SPECIAL_FILES = {"invite.py", "sticky.py"}


@dataclass
class Endpoint:
    resource: str
    method: str
    path: str
    deprecated: bool


@dataclass
class Param:
    name: str
    flag: str


@dataclass
class Operation:
    name: str
    method: str
    path: str
    deprecated: bool
    params: List[Param]


@dataclass
class Resource:
    name: str
    ops: List[Operation]


@dataclass
class CommandTree:
    version: int
    base_path: str
    resources: List[Resource]


def iter_path_blocks(text: str) -> Iterable[str]:
    idx = 0
    while True:
        start = text.find("path(", idx)
        if start == -1:
            return
        depth = 0
        i = start
        while i < len(text):
            if text[i] == "(":
                depth += 1
            elif text[i] == ")":
                depth -= 1
                if depth == 0:
                    yield text[start : i + 1]
                    idx = i + 1
                    break
            i += 1
        else:
            return


def parse_paths(file_path: str) -> List[Tuple[str, List[str]]]:
    with open(file_path, "r", encoding="utf-8") as f:
        text = f.read()

    entries: List[Tuple[str, List[str]]] = []
    for block in iter_path_blocks(text):
        path_match = PATH_CALL_RE.search(block)
        if not path_match:
            continue
        methods_match = METHODS_RE.search(block)
        if not methods_match:
            continue
        methods = METHOD_TOKEN_RE.findall(methods_match.group(1))
        if not methods:
            continue
        path = path_match.group(1)
        entries.append((path, [m.upper() for m in methods]))
    return entries


def find_resource(path: str, file_name: str) -> Tuple[str, bool]:
    if file_name == "work_item.py":
        if "/issues/" in path and "/work-items/" not in path:
            return "issue", True
        return "work-item", False
    stem = os.path.splitext(file_name)[0]
    return stem.replace("_", "-"), False


def extract_params(path: str) -> List[Param]:
    params: List[Param] = []
    seen = set()
    for match in PARAM_RE.findall(path):
        token = match[1:-1]
        name = token.split(":", 1)[1] if ":" in token else token
        if name in seen:
            continue
        seen.add(name)
        params.append(Param(name=name, flag=name.replace("_", "-")))
    return params


def is_param_segment(seg: str) -> bool:
    return seg.startswith("<") and seg.endswith(">")


def op_name(path: str, method: str, resource: str) -> str:
    if "project_identifier" in path and "issue_identifier" in path and method == "GET":
        return "by-identifier"

    segments = [s for s in path.strip("/").split("/") if s]
    plain_segments = [s for s in segments if not is_param_segment(s)]

    base_segments = RESOURCE_SEGMENTS.get(resource, [])
    action_segments = plain_segments
    base_idx = None
    for base in base_segments:
        if base in plain_segments:
            base_idx = plain_segments.index(base)
            break
    if base_idx is not None:
        action_segments = plain_segments[base_idx + 1 :]
    else:
        action_segments = [s for s in plain_segments if s not in IGNORE_PREFIXES]

    is_detail = segments[-1] if segments else ""
    is_detail = is_param_segment(is_detail)

    prefix_segments = action_segments[:-1] if len(action_segments) > 1 else []
    last_action = action_segments[-1] if action_segments else ""

    def with_prefix(action: str) -> str:
        if not prefix_segments:
            return action
        return f"{'-'.join(prefix_segments)}-{action}"

    if action_segments:
        if last_action in {"search", "suggest", "count", "unread-count"} and method == "GET":
            return with_prefix(last_action)
        if last_action == "archive" and method == "DELETE":
            return with_prefix("unarchive")
        if last_action == "unarchive":
            return with_prefix("unarchive")

        if method == "GET":
            return with_prefix(f"{last_action}-get" if is_detail else f"{last_action}-list")
        if method == "POST":
            if last_action in {"archive", "transfer-issues"}:
                return with_prefix(last_action)
            return with_prefix(f"{last_action}-create")
        if method == "PATCH":
            return with_prefix(f"{last_action}-update")
        if method == "DELETE":
            return with_prefix(f"{last_action}-delete")

    if method == "GET":
        return "get" if is_detail else "list"
    if method == "POST":
        return "create"
    if method == "PATCH":
        return "update"
    if method == "DELETE":
        return "delete"

    return method.lower()


def add_invite_endpoints(endpoints: List[Endpoint]) -> None:
    base = "workspaces/<str:slug>/invitations/"
    endpoints.extend(
        [
            Endpoint("invite", "GET", base, False),
            Endpoint("invite", "POST", base, False),
            Endpoint("invite", "GET", base + "<uuid:pk>/", False),
            Endpoint("invite", "PATCH", base + "<uuid:pk>/", False),
            Endpoint("invite", "DELETE", base + "<uuid:pk>/", False),
        ]
    )


def add_sticky_endpoints(endpoints: List[Endpoint]) -> None:
    base = "workspaces/<str:slug>/stickies/"
    endpoints.extend(
        [
            Endpoint("sticky", "GET", base, False),
            Endpoint("sticky", "POST", base, False),
            Endpoint("sticky", "GET", base + "<uuid:pk>/", False),
            Endpoint("sticky", "PATCH", base + "<uuid:pk>/", False),
            Endpoint("sticky", "DELETE", base + "<uuid:pk>/", False),
        ]
    )


def build_command_tree(api_dir: str) -> CommandTree:
    endpoints: List[Endpoint] = []
    for file_name in os.listdir(api_dir):
        if not file_name.endswith(".py"):
            continue
        file_path = os.path.join(api_dir, file_name)
        if file_name in SPECIAL_FILES:
            continue
        for path, methods in parse_paths(file_path):
            resource, deprecated = find_resource(path, file_name)
            for method in methods:
                endpoints.append(Endpoint(resource, method, path, deprecated))

    add_invite_endpoints(endpoints)
    add_sticky_endpoints(endpoints)

    resources: Dict[str, List[Operation]] = {}
    seen = set()
    for endpoint in endpoints:
        key = (endpoint.resource, endpoint.method, endpoint.path)
        if key in seen:
            continue
        seen.add(key)

        params = extract_params(endpoint.path)
        name = op_name(endpoint.path, endpoint.method, endpoint.resource)

        ops = resources.setdefault(endpoint.resource, [])
        name = ensure_unique_name(name, endpoint.method, ops)
        ops.append(
            Operation(
                name=name,
                method=endpoint.method,
                path=endpoint.path,
                deprecated=endpoint.deprecated,
                params=params,
            )
        )

    out_resources = [
        Resource(name=resource, ops=sorted(ops, key=lambda op: op.name))
        for resource, ops in sorted(resources.items())
    ]

    return CommandTree(version=1, base_path="/api/v1", resources=out_resources)


def ensure_unique_name(name: str, method: str, ops: List[Operation]) -> str:
    existing = {op.name for op in ops}
    if name not in existing:
        return name
    candidate = f"{name}-{method.lower()}"
    if candidate not in existing:
        return candidate
    idx = 2
    while True:
        candidate = f"{name}-{idx}"
        if candidate not in existing:
            return candidate
        idx += 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate Plane CLI command tree from Plane API URLs")
    parser.add_argument("--plane-repo", default=os.getenv("PLANE_REPO_PATH"), help="Path to Plane repo")
    parser.add_argument("--out", default="schemas/command_tree.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.plane_repo:
        raise SystemExit("--plane-repo or PLANE_REPO_PATH is required")

    api_dir = os.path.join(args.plane_repo, "apps", "api", "plane", "api", "urls")
    if not os.path.isdir(api_dir):
        raise SystemExit(f"API urls dir not found: {api_dir}")

    tree = build_command_tree(api_dir)

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(tree, f, indent=2, default=lambda obj: obj.__dict__)
    print(args.out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
