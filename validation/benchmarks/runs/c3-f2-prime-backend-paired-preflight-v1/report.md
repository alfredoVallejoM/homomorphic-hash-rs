# Benchmark algesum-c3-f1-f2-closure-v1-f2-prime-backend-paired-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f2-prime-backend-paired-fp251-portable-paired-n8` | 8 field-elements | 29.513 | [29.504, 29.521] | 29.575 | Precise |
| `f2-prime-backend-paired-fp251-forced-paired-n8` | 8 field-elements | 29.535 | [29.511, 29.559] | 29.590 | Precise |
| `f2-prime-backend-paired-goldilocks-portable-paired-n8` | 8 field-elements | 115.071 | [82.740, 147.402] | 150.784 | Inconclusive |
| `f2-prime-backend-paired-goldilocks-forced-paired-n8` | 8 field-elements | 108.632 | [76.507, 140.757] | 140.893 | Inconclusive |
| `f2-prime-backend-paired-fp256-generic-portable-paired-n8` | 8 field-elements | 1469.807 | [1463.996, 1475.617] | 1492.872 | Precise |
| `f2-prime-backend-paired-fp256-generic-portable-paired-n64` | 64 field-elements | 7059.531 | [7057.094, 7061.969] | 7213.008 | Precise |
| `f2-prime-backend-paired-fp256-generic-forced-paired-n64` | 64 field-elements | 11506.297 | [11500.781, 11511.812] | 11651.256 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `f2-prime-backend-paired-fp251-forced-paired-n8` | `f2-prime-backend-paired-fp251-portable-paired-n8` | 1.0007 | [0.9996, 1.0019] |
| `f2-prime-backend-paired-goldilocks-forced-paired-n8` | `f2-prime-backend-paired-goldilocks-portable-paired-n8` | 1.1101 | [0.5190, 1.7012] |
| `f2-prime-backend-paired-fp256-generic-forced-paired-n64` | `f2-prime-backend-paired-fp256-generic-portable-paired-n64` | 1.6299 | [1.6297, 1.6301] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
