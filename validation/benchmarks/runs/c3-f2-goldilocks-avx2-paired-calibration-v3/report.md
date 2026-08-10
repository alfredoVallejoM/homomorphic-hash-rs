# Benchmark algesum-c3-f1-f2-closure-v1-goldilocks-avx2-paired-calibration-v3

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f2-goldilocks-portable-n8-calibration-v3` | 8 field-elements | 83.066 | [82.988, 83.181] | 85.899 | Precise |
| `f2-goldilocks-avx2-n8-calibration-v3` | 8 field-elements | 79.799 | [79.667, 79.925] | 93.091 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `f2-goldilocks-avx2-n8-calibration-v3` | `f2-goldilocks-portable-n8-calibration-v3` | 0.9602 | [0.9591, 0.9621] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
