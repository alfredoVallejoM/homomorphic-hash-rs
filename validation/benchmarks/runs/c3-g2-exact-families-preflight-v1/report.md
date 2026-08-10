# Benchmark algesum-c3-g1-g2-v1-g2-exact-families-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `g2-exact-families-path-n6` | 6 vertices | 2070.536 | [1895.339, 2245.734] | 2345.351 | Precise |
| `g2-exact-families-cycle-n6` | 6 vertices | 5859.396 | [5794.521, 5924.271] | 6018.027 | Precise |
| `g2-exact-families-components-n6` | 6 vertices | 7661.750 | [7179.375, 8144.125] | 8321.799 | Precise |
| `g2-exact-families-regular-n6` | 6 vertices | 6846.883 | [6774.984, 6918.781] | 9236.610 | Precise |
| `g2-exact-families-star-n10` | 10 vertices | 41593.550 | [41468.100, 41719.000] | 43235.910 | Precise |

## Telemetría exacta de grafos

| Celda | Outcome | Presupuesto | Explorados | Hojas | Profundidad | Ruta | Límite agotado |
|---|---|---:|---:|---:|---:|---|---|
| `g2-exact-families-path-n6` | exact | 1000000 | 0 | 1 | 0 | exact-refinement-discrete | — |
| `g2-exact-families-cycle-n6` | exact | 1000000 | 7 | 4 | 2 | individualization-refinement | — |
| `g2-exact-families-components-n6` | exact | 1000000 | 14 | 8 | 2 | weak-component-decomposition | — |
| `g2-exact-families-regular-n6` | exact | 1000000 | 0 | 1 | 0 | exact-refinement-discrete | — |
| `g2-exact-families-star-n10` | exact | 1000000 | 129 | 37 | 8 | individualization-refinement | — |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
