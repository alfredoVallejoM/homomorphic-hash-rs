# Benchmark algesum-c3-p0-v1-f1-static-binary-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f1-static-binary-gf2-128-add-n1` | 1 field-elements | 63.282 | [63.005, 63.559] | 92.420 | Precise |
| `f1-static-binary-gf2-128-mul-n2` | 2 field-elements | 573.214 | [414.059, 732.369] | 764.488 | Inconclusive |
| `f1-static-binary-gf2-128-square-n3` | 3 field-elements | 127.211 | [127.201, 127.222] | 138.528 | Precise |
| `f1-static-binary-gf2-128-invert-n4` | 4 field-elements | 168191.250 | [167034.500, 169348.000] | 171268.675 | Precise |
| `f1-static-binary-gf2-128-wire-n7` | 7 field-elements | 139.593 | [88.655, 190.530] | 194.815 | Inconclusive |
| `f1-static-binary-gf2-256-hh-add-n8` | 8 field-elements | 533.884 | [533.619, 534.148] | 547.095 | Precise |
| `f1-static-binary-gf2-256-hh-mul-n15` | 15 field-elements | 18156.781 | [18101.250, 18212.312] | 18500.222 | Precise |
| `f1-static-binary-gf2-256-hh-square-n16` | 16 field-elements | 1485.480 | [1483.094, 1487.867] | 1513.428 | Precise |
| `f1-static-binary-gf2-256-hh-invert-n31` | 31 field-elements | 7188898.000 | [7159590.000, 7218206.000] | 7316332.250 | Precise |
| `f1-static-binary-gf2-256-hh-wire-n32` | 32 field-elements | 1503.424 | [1026.434, 1980.414] | 2213.910 | Inconclusive |
| `f1-static-binary-gf2-256-alt-add-n63` | 63 field-elements | 3983.680 | [3981.484, 3985.875] | 4096.534 | Precise |
| `f1-static-binary-gf2-256-alt-mul-n64` | 64 field-elements | 85406.625 | [80658.500, 90154.750] | 94539.412 | Precise |
| `f1-static-binary-gf2-256-alt-square-n255` | 255 field-elements | 18759.625 | [14129.625, 23389.625] | 23706.825 | Precise |
| `f1-static-binary-gf2-256-alt-invert-n256` | 256 field-elements | 54035879.000 | [52580226.000, 55491532.000] | 56189233.050 | Precise |
| `f1-static-binary-gf2-256-alt-wire-n4096` | 4096 field-elements | 254315.000 | [252467.000, 256163.000] | 265793.300 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
