# Benchmark algesum-c3-p0-v1-f2-static-prime-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f2-static-prime-fp251-add-n1` | 1 field-elements | 6.616 | [5.372, 7.859] | 7.878 | Precise |
| `f2-static-prime-fp251-mul-n2` | 2 field-elements | 15.091 | [11.087, 19.096] | 62.042 | Inconclusive |
| `f2-static-prime-fp251-square-n3` | 3 field-elements | 14.254 | [14.252, 14.257] | 14.302 | Precise |
| `f2-static-prime-fp251-invert-n4` | 4 field-elements | 131.602 | [96.212, 166.992] | 170.210 | Inconclusive |
| `f2-static-prime-fp251-wire-n7` | 7 field-elements | 15.776 | [15.766, 15.786] | 20.736 | Precise |
| `f2-static-prime-goldilocks-add-n8` | 8 field-elements | 56.340 | [56.223, 56.458] | 98.284 | Precise |
| `f2-static-prime-goldilocks-mul-n15` | 15 field-elements | 200.392 | [141.947, 258.836] | 263.400 | Inconclusive |
| `f2-static-prime-goldilocks-square-n16` | 16 field-elements | 278.079 | [278.013, 278.145] | 282.657 | Precise |
| `f2-static-prime-goldilocks-invert-n31` | 31 field-elements | 39179.500 | [39179.000, 39180.000] | 66308.881 | Precise |
| `f2-static-prime-goldilocks-wire-n32` | 32 field-elements | 262.465 | [175.433, 349.497] | 386.397 | Inconclusive |
| `f2-static-prime-fp256-generic-add-n63` | 63 field-elements | 8006.406 | [8001.031, 8011.781] | 8252.939 | Precise |
| `f2-static-prime-fp256-generic-mul-n64` | 64 field-elements | 8996.297 | [6806.281, 11186.312] | 11330.464 | Precise |
| `f2-static-prime-fp256-generic-square-n255` | 255 field-elements | 34966.500 | [26227.625, 43705.375] | 45123.081 | Precise |
| `f2-static-prime-fp256-generic-invert-n256` | 256 field-elements | 4193079.000 | [4192598.000, 4193560.000] | 4237457.800 | Precise |
| `f2-static-prime-fp256-generic-wire-n4096` | 4096 field-elements | 579228.000 | [578047.000, 580409.000] | 583118.200 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
