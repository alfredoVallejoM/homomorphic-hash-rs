from pathlib import Path
import hashlib
import json

root = Path(__file__).resolve().parents[2]
output = root / "validation" / "f6" / "corpora" / "simple-n8.g6"
metadata = root / "validation" / "f6" / "corpora" / "simple-n8.json"
output.parent.mkdir(parents=True, exist_ok=True)

lines = sorted(graph.graph6_string() for graph in graphs.nauty_geng("8"))
payload = ("\n".join(lines) + "\n").encode("ascii")
output.write_bytes(payload)
metadata.write_text(
    json.dumps(
        {
            "schema_version": int(1),
            "generator": "SageMath graphs.nauty_geng",
            "sage_version": str(sage.version.version),
            "vertices": int(8),
            "graph_count": int(len(lines)),
            "sha256": hashlib.sha256(payload).hexdigest(),
            "format": "graph6, one graph per line, no header",
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
print(f"wrote {len(lines)} graphs to {output}")
