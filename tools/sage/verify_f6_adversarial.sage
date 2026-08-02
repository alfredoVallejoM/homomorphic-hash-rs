from pathlib import Path
import json

root = Path(__file__).resolve().parents[2]
output = root / "validation" / "f6" / "corpora" / "adversarial-oracle.json"


def cfi_k4(twisted_edge=None):
    base_edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
    incident = [
        [edge for edge, (left, right) in enumerate(base_edges) if vertex in (left, right)]
        for vertex in range(4)
    ]
    graph = Graph()
    outer = [[None for _ in range(3)] for _ in range(4)]
    next_vertex = 0
    for vertex in range(4):
        for local_edge in range(3):
            outer[vertex][local_edge] = [next_vertex, next_vertex + 1]
            graph.add_vertices([next_vertex, next_vertex + 1])
            next_vertex += 2
    for vertex in range(4):
        for mask in range(8):
            if mask.bit_count() % 2:
                continue
            middle = next_vertex
            next_vertex += 1
            graph.add_vertex(middle)
            for local_edge in range(3):
                graph.add_edge(middle, outer[vertex][local_edge][(mask >> local_edge) & 1])
    for edge, (left, right) in enumerate(base_edges):
        left_local = incident[left].index(edge)
        right_local = incident[right].index(edge)
        twist = int(twisted_edge == edge)
        for bit in range(2):
            graph.add_edge(
                outer[left][left_local][bit],
                outer[right][right_local][bit ^ twist],
            )
    return graph


cycle6 = graphs.CycleGraph(6)
two_triangles = graphs.CycleGraph(3).disjoint_union(graphs.CycleGraph(3))
rook = graphs.RookGraph([4, 4])
shrikhande = graphs.ShrikhandeGraph()
cfi_even = cfi_k4()
cfi_odd = cfi_k4(0)

results = {
    "schema_version": int(1),
    "oracle": "SageMath Graph.is_isomorphic",
    "sage_version": str(sage.version.version),
    "families": [
        {
            "name": "C6 versus 2C3",
            "left_vertices": int(cycle6.order()),
            "right_vertices": int(two_triangles.order()),
            "isomorphic": bool(cycle6.is_isomorphic(two_triangles)),
        },
        {
            "name": "Shrikhande versus 4x4 rook",
            "left_vertices": int(shrikhande.order()),
            "right_vertices": int(rook.order()),
            "isomorphic": bool(shrikhande.is_isomorphic(rook)),
        },
        {
            "name": "CFI(K4) even versus one twisted edge",
            "left_vertices": int(cfi_even.order()),
            "right_vertices": int(cfi_odd.order()),
            "left_edges": int(cfi_even.size()),
            "right_edges": int(cfi_odd.size()),
            "isomorphic": bool(cfi_even.is_isomorphic(cfi_odd)),
        },
    ],
}
output.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n", encoding="utf-8")
print(output)
