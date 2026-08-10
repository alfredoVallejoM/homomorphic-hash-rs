# Benchmark algesum-c3-f1-f2-closure-v1-f1-binary-backend-paired-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f1-binary-backend-paired-gf2-128-portable-paired-n8` | 8 field-elements | 2800.172 | [2732.461, 2867.883] | 2988.333 | Precise |
| `f1-binary-backend-paired-gf2-128-forced-paired-n8` | 8 field-elements | 271.993 | [271.885, 272.101] | 278.834 | Precise |
| `f1-binary-backend-paired-gf2-256-hh-portable-paired-n8` | 8 field-elements | 11506.391 | [11398.656, 11614.125] | 12385.794 | Precise |
| `f1-binary-backend-paired-gf2-256-hh-forced-paired-n8` | 8 field-elements | 716.870 | [712.125, 721.615] | 752.461 | Precise |
| `f1-binary-backend-paired-gf2-256-alt-portable-paired-n8` | 8 field-elements | 6593.805 | [6407.297, 6780.312] | 6953.991 | Precise |
| `f1-binary-backend-paired-gf2-256-alt-portable-paired-n64` | 64 field-elements | 75090.000 | [56383.250, 93796.750] | 96205.275 | Precise |
| `f1-binary-backend-paired-gf2-256-alt-forced-paired-n64` | 64 field-elements | 4450.180 | [3467.438, 5432.922] | 5516.177 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `f1-binary-backend-paired-gf2-128-forced-paired-n8` | `f1-binary-backend-paired-gf2-128-portable-paired-n8` | 0.0972 | [0.0948, 0.0996] |
| `f1-binary-backend-paired-gf2-256-hh-forced-paired-n8` | `f1-binary-backend-paired-gf2-256-hh-portable-paired-n8` | 0.0623 | [0.0621, 0.0625] |
| `f1-binary-backend-paired-gf2-256-alt-forced-paired-n64` | `f1-binary-backend-paired-gf2-256-alt-portable-paired-n64` | 0.0597 | [0.0579, 0.0615] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
