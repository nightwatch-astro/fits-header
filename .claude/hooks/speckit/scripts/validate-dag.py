#!/usr/bin/env python3
"""Validate tasks.md [graph] section is acyclic and report ready tasks.

Usage: validate-dag.py <tasks.md> [--closed T001,T002,T003]

Exit 0 = valid DAG, exit 1 = cycle detected, exit 2 = parse error.
"""

import argparse
import collections
import re
import sys


def parse_graph(text: str) -> dict[str, list[str]]:
    """Extract [graph.TXXX] entries from TOML block in tasks.md.

    The blocked_by array body is captured with ``\\[([^\\]]*)\\]`` rather than
    ``\\[(.*?)\\]``. The character-class form spans newlines (``.`` does not
    without re.DOTALL), so a multi-line TOML array such as::

        [graph.T002]
        blocked_by = [
            "T001",
            "T003",
        ]

    is parsed in full. The previous pattern stopped at the first newline and
    captured an empty dependency list, silently MISSING real cycles whose
    blocked_by arrays were formatted across multiple lines.
    """
    graph: dict[str, list[str]] = {}
    for m in re.finditer(
        r"\[graph\.(T\d+)\]\s*\nblocked_by\s*=\s*\[([^\]]*)\]", text
    ):
        task = m.group(1)
        deps = [d.strip().strip('"') for d in m.group(2).split(",") if d.strip()]
        graph[task] = deps
    return graph


def kahns_algorithm(
    graph: dict[str, list[str]], closed: set[str]
) -> tuple[list[str], list[str], list[str]]:
    """Run Kahn's algorithm. Returns (topo_order, ready, cycle_members)."""
    open_tasks = {t for t in graph if t not in closed}

    # Build adjacency and in-degree for open tasks only
    in_degree: dict[str, int] = {t: 0 for t in open_tasks}
    adj: dict[str, list[str]] = collections.defaultdict(list)

    for t in open_tasks:
        for dep in graph.get(t, []):
            if dep in open_tasks:  # only count open blockers
                adj[dep].append(t)
                in_degree[t] += 1

    queue = sorted(t for t, d in in_degree.items() if d == 0)
    ready = list(queue)
    topo_order: list[str] = []

    while queue:
        node = queue.pop(0)
        topo_order.append(node)
        for dependent in sorted(adj[node]):
            in_degree[dependent] -= 1
            if in_degree[dependent] == 0:
                queue.append(dependent)

    if len(topo_order) < len(open_tasks):
        cycle_members = sorted(t for t, d in in_degree.items() if d > 0)
        return topo_order, ready, cycle_members

    return topo_order, ready, []


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tasks_file", help="Path to tasks.md")
    parser.add_argument(
        "--closed",
        default="",
        help="Comma-separated list of closed task IDs (e.g. T001,T002,T003)",
    )
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    args = parser.parse_args()

    try:
        text = open(args.tasks_file).read()
    except FileNotFoundError:
        print(f"File not found: {args.tasks_file}", file=sys.stderr)
        sys.exit(2)

    graph = parse_graph(text)
    if not graph:
        print("No [graph] section found in tasks.md", file=sys.stderr)
        sys.exit(2)

    closed = {t.strip() for t in args.closed.split(",") if t.strip()}
    topo_order, ready, cycle_members = kahns_algorithm(graph, closed)

    open_count = len([t for t in graph if t not in closed])

    if args.json:
        import json

        result = {
            "valid": len(cycle_members) == 0,
            "total": len(graph),
            "closed": len(closed),
            "open": open_count,
            "ready": ready,
            "topo_order": topo_order,
            "cycle": cycle_members,
        }
        print(json.dumps(result))
        sys.exit(1 if cycle_members else 0)

    if cycle_members:
        print(f"CYCLE detected among: {', '.join(cycle_members)}", file=sys.stderr)
        sys.exit(1)

    print(f"DAG valid: {len(graph)} total, {open_count} open, {len(ready)} ready")
    if ready:
        print(f"Ready now: {', '.join(ready)}")


if __name__ == "__main__":
    main()
