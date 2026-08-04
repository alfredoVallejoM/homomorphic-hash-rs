"""Independent SageMath oracle for the Phase 6.G5 adversarial family."""

import json

from sage.all import Graph, graphs
from sage.version import version as sage_version


def exact_color_refinement(graph):
    """Independent exact 1-WL partition for an unlabelled simple graph."""
    colors = {vertex: 0 for vertex in graph.vertices(sort=True)}
    while True:
        signatures = {
            vertex: (colors[vertex], tuple(sorted(colors[n] for n in graph.neighbors(vertex))))
            for vertex in graph.vertices(sort=True)
        }
        palette = {signature: index for index, signature in enumerate(sorted(set(signatures.values())))}
        refined = {vertex: palette[signature] for vertex, signature in signatures.items()}
        if refined == colors:
            return refined
        colors = refined


def disconnected_cycles(lengths):
    graph = Graph()
    offset = 0
    for length in lengths:
        graph.add_vertices(range(offset, offset + length))
        graph.add_edges(
            (offset + index, offset + ((index + 1) % length))
            for index in range(length)
        )
        offset += length
    return graph


def rook_graph_4x4():
    graph = Graph()
    graph.add_vertices(range(16))
    graph.add_edges(
        (left, right)
        for left in range(16)
        for right in range(left + 1, 16)
        if left // 4 == right // 4 or left % 4 == right % 4
    )
    return graph


def shrikhande_graph():
    graph = Graph()
    graph.add_vertices(range(16))
    generators = [(1, 0), (3, 0), (0, 1), (0, 3), (1, 1), (3, 3)]
    graph.add_edges(
        (
            row * 4 + column,
            ((row + delta_row) % 4) * 4 + ((column + delta_column) % 4),
        )
        for row in range(4)
        for column in range(4)
        for delta_row, delta_column in generators
    )
    return graph


def main():
    checked = 0
    for size in range(6, 41):
        split = max(3, size // 2)
        if size - split < 3:
            continue
        connected = graphs.CycleGraph(size)
        disconnected = disconnected_cycles([split, size - split])
        assert connected.degree_sequence() == disconnected.degree_sequence()
        assert len(set(exact_color_refinement(connected).values())) == 1
        assert len(set(exact_color_refinement(disconnected).values())) == 1
        assert not connected.is_isomorphic(disconnected)
        assert connected.canonical_label() != disconnected.canonical_label()
        checked += 1

    shrikhande = shrikhande_graph()
    rook = rook_graph_4x4()
    assert shrikhande.degree_sequence() == rook.degree_sequence() == [6] * 16
    assert len(set(exact_color_refinement(shrikhande).values())) == 1
    assert len(set(exact_color_refinement(rook).values())) == 1
    assert not shrikhande.is_isomorphic(rook)
    assert shrikhande.canonical_label() != rook.canonical_label()

    print(
        json.dumps(
            {
                "ok": True,
                "oracle": "SageMath Graph.is_isomorphic + exact Python 1-WL",
                "sage_version": sage_version,
                "non_isomorphic_regular_pairs": int(checked),
                "non_isomorphic_strongly_regular_pairs": int(1),
                "minimum_collision_vertices": int(6),
            },
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
