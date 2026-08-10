# Benchmark algesum-c3-g1-g2-v1-g1-linear-graph-pipeline-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `g1-linear-graph-pipeline-build-prepare-n64` | 64 vertices | 398.182 | [383.436, 412.928] | 586.308 | Precise |
| `g1-linear-graph-pipeline-field-n64` | 64 vertices | 508.581 | [497.857, 519.305] | 637.797 | Precise |
| `g1-linear-graph-pipeline-hybrid-n64` | 64 vertices | 2175.766 | [2172.828, 2178.703] | 2246.506 | Precise |
| `g1-linear-graph-pipeline-parallel-n64` | 64 vertices | 4682.797 | [4173.781, 5191.812] | 8312.329 | Precise |
| `g1-linear-graph-pipeline-memory-n1024` | 1024 vertices | 1489.210 | [1480.321, 1498.099] | 1566.218 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
