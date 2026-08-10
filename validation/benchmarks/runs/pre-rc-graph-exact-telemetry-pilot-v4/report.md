# Benchmark pre-rc-graph-exact-telemetry-pilot-v4

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `graph-exact-path-8` | 8 vertices | 2291.250 | [1930.500, 2340.375] | 2428.675 | Precise |
| `graph-exact-path-16` | 16 vertices | 2002.688 | [1960.188, 2348.062] | 2699.837 | Precise |
| `graph-exact-cycle-8` | 8 vertices | 6588.375 | [5674.250, 6600.250] | 7069.500 | Precise |
| `graph-exact-cycle-12` | 12 vertices | 6431.583 | [6304.667, 7959.083] | 8096.683 | Precise |
| `graph-exact-cycle-16` | 16 vertices | 7428.438 | [7254.688, 9067.562] | 9192.087 | Precise |
| `graph-inconclusive-cycle-16` | 16 vertices | 360.812 | [358.000, 367.375] | 414.062 | Precise |

## Telemetría exacta de grafos

| Celda | Outcome | Presupuesto | Explorados | Hojas | Profundidad | Ruta | Límite agotado |
|---|---|---:|---:|---:|---:|---|---|
| `graph-exact-path-8` | exact | 1000000 | 0 | 1 | 0 | exact-refinement-discrete | — |
| `graph-exact-path-16` | exact | 1000000 | 0 | 1 | 0 | exact-refinement-discrete | — |
| `graph-exact-cycle-8` | exact | 1000000 | 7 | 4 | 2 | individualization-refinement | — |
| `graph-exact-cycle-12` | exact | 1000000 | 7 | 4 | 2 | individualization-refinement | — |
| `graph-exact-cycle-16` | exact | 1000000 | 7 | 4 | 2 | individualization-refinement | — |
| `graph-inconclusive-cycle-16` | inconclusive | 1 | 1 | 0 | 0 | individualization-refinement | search-nodes |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
