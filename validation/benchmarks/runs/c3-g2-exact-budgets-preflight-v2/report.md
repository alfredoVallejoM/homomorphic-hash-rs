# Benchmark algesum-c3-g1-g2-v1-g2-exact-budgets-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `g2-exact-budgets-nodes-p1-n6` | 6 vertices | 417.387 | [416.283, 418.491] | 423.962 | Precise |
| `g2-exact-budgets-cells-p64-n6` | 6 vertices | 5908.573 | [5878.250, 5938.896] | 6012.781 | Precise |
| `g2-exact-budgets-bytes-p1-n10` | 10 vertices | 180.646 | [180.384, 180.907] | 206.436 | Precise |
| `g2-exact-budgets-depth-p1024-n10` | 10 vertices | 6316.075 | [5706.750, 6925.400] | 7798.376 | Precise |

## Telemetría exacta de grafos

| Celda | Outcome | Presupuesto | Explorados | Hojas | Profundidad | Ruta | Límite agotado |
|---|---|---:|---:|---:|---:|---|---|
| `g2-exact-budgets-nodes-p1-n6` | inconclusive | 1 | 1 | 0 | 0 | individualization-refinement | search-nodes |
| `g2-exact-budgets-cells-p64-n6` | exact | 1000000 | 7 | 4 | 2 | individualization-refinement | — |
| `g2-exact-budgets-bytes-p1-n10` | inconclusive | 1000000 | 0 | 0 | 0 | exact-refinement-discrete | retained-bytes |
| `g2-exact-budgets-depth-p1024-n10` | exact | 1000000 | 7 | 4 | 2 | individualization-refinement | — |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
