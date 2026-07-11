#!/usr/bin/env python3
"""Tests for validate-dag.py parse_graph + cycle detection.

Phase-1 audit remediation: the blocked_by array capture must be line-tolerant.
The previous pattern ``\\[(.*?)\\]`` stopped at the first newline (``.`` does
not match newlines without re.DOTALL), so a multi-line TOML array captured an
empty dependency list and real cycles were silently MISSED.

The module name contains a hyphen, so it is loaded via importlib rather than a
plain import.
"""
import importlib.util
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))


def _load_module(name, filename):
    path = os.path.join(HERE, filename)
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


vd = _load_module("validate_dag", "validate-dag.py")


# Multi-line blocked_by array: T001 -> T002 -> T001 is a 2-cycle. With the old
# single-line regex these arrays parse as empty and the cycle is missed.
MULTILINE_CYCLE = """\
Some preamble text.

[graph.T001]
blocked_by = [
    "T002",
]

[graph.T002]
blocked_by = [
    "T001",
]
"""

# Single-line form of the same cycle (must still work after the fix).
SINGLELINE_CYCLE = """\
[graph.T001]
blocked_by = ["T002"]

[graph.T002]
blocked_by = ["T001"]
"""

# Multi-line acyclic graph: T002 depends on T001, T003 depends on T001+T002.
MULTILINE_ACYCLIC = """\
[graph.T001]
blocked_by = []

[graph.T002]
blocked_by = [
    "T001",
]

[graph.T003]
blocked_by = [
    "T001",
    "T002",
]
"""


def test_parse_graph_multiline_array_captures_all_deps():
    graph = vd.parse_graph(MULTILINE_ACYCLIC)
    assert graph["T001"] == []
    assert graph["T002"] == ["T001"]
    # The critical assertion: a 2-element multi-line array is fully captured,
    # not truncated at the first newline.
    assert graph["T003"] == ["T001", "T002"], graph["T003"]


def test_multiline_cycle_is_detected():
    graph = vd.parse_graph(MULTILINE_CYCLE)
    # Both deps must be captured across newlines.
    assert graph["T001"] == ["T002"], graph["T001"]
    assert graph["T002"] == ["T001"], graph["T002"]
    _, _, cycle = vd.kahns_algorithm(graph, closed=set())
    assert sorted(cycle) == ["T001", "T002"], cycle


def test_singleline_cycle_still_detected():
    graph = vd.parse_graph(SINGLELINE_CYCLE)
    assert graph["T001"] == ["T002"]
    assert graph["T002"] == ["T001"]
    _, _, cycle = vd.kahns_algorithm(graph, closed=set())
    assert sorted(cycle) == ["T001", "T002"], cycle


def test_multiline_acyclic_no_cycle():
    graph = vd.parse_graph(MULTILINE_ACYCLIC)
    topo, ready, cycle = vd.kahns_algorithm(graph, closed=set())
    assert cycle == [], cycle
    assert "T001" in ready  # only node with no open blockers
    assert set(topo) == {"T001", "T002", "T003"}


def test_empty_multiline_blocked_by():
    text = "[graph.T001]\nblocked_by = [\n]\n"
    graph = vd.parse_graph(text)
    assert graph["T001"] == []


def test_three_node_multiline_cycle():
    text = """\
[graph.T001]
blocked_by = [
    "T003",
]

[graph.T002]
blocked_by = [
    "T001",
]

[graph.T003]
blocked_by = [
    "T002",
]
"""
    graph = vd.parse_graph(text)
    _, _, cycle = vd.kahns_algorithm(graph, closed=set())
    assert sorted(cycle) == ["T001", "T002", "T003"], cycle


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
