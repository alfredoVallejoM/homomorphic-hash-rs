# Benchmark algesum-c3-f1-f2-closure-v1-f2-prime-x86-explicit-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f2-prime-x86-explicit-fp251-portable-x86-n8` | 8 field-elements | 29.515 | [29.502, 29.528] | 33.971 | Precise |
| `f2-prime-x86-explicit-fp251-avx2-n8` | 8 field-elements | 40.241 | [40.232, 40.249] | 47.163 | Precise |
| `f2-prime-x86-explicit-goldilocks-portable-x86-n8` | 8 field-elements | 115.021 | [82.977, 147.065] | 147.100 | Inconclusive |
| `f2-prime-x86-explicit-goldilocks-avx2-n8` | 8 field-elements | 109.562 | [78.250, 140.873] | 142.889 | Inconclusive |
| `f2-prime-x86-explicit-fp256-generic-portable-x86-n8` | 8 field-elements | 1453.982 | [1453.133, 1454.832] | 1468.740 | Precise |
| `f2-prime-x86-explicit-fp256-generic-portable-x86-n64` | 64 field-elements | 7139.891 | [7068.375, 7211.406] | 7240.217 | Precise |
| `f2-prime-x86-explicit-fp256-generic-bmi2-n64` | 64 field-elements | 11791.516 | [11701.406, 11881.625] | 12470.530 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `f2-prime-x86-explicit-fp251-avx2-n8` | `f2-prime-x86-explicit-fp251-portable-x86-n8` | 1.3634 | [1.3631, 1.3637] |
| `f2-prime-x86-explicit-goldilocks-avx2-n8` | `f2-prime-x86-explicit-goldilocks-portable-x86-n8` | 0.9505 | [0.9430, 0.9579] |
| `f2-prime-x86-explicit-fp256-generic-bmi2-n64` | `f2-prime-x86-explicit-fp256-generic-portable-x86-n64` | 1.6518 | [1.6226, 1.6810] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
