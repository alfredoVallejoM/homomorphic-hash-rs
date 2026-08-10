# Benchmark algesum-c3-f3-s3-v1-f3-derived-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f3-derived-scan-prefix-inclusive-n1` | 1 field-elements | 901.648 | [859.648, 943.648] | 958.759 | Precise |
| `f3-derived-scan-prefix-exclusive-n3` | 3 field-elements | 4143.844 | [4069.844, 4217.844] | 4276.991 | Precise |
| `f3-derived-scan-suffix-inclusive-n8` | 8 field-elements | 10593.391 | [10366.312, 10820.469] | 10927.106 | Precise |
| `f3-derived-scan-suffix-exclusive-n16` | 16 field-elements | 22759.156 | [22610.375, 22907.938] | 23251.969 | Precise |
| `f3-derived-batch-invert-n63` | 63 field-elements | 507394.500 | [388367.000, 626422.000] | 629735.600 | Precise |
| `f3-derived-powers-n255` | 255 field-elements | 214077.500 | [203825.000, 224330.000] | 225212.175 | Precise |
| `f3-derived-mask-n4096` | 4096 field-elements | 121.746 | [107.626, 135.865] | 192.554 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
