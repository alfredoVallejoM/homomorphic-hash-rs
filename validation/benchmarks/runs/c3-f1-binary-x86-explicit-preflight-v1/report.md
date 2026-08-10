# Benchmark algesum-c3-f1-f2-closure-v1-f1-binary-x86-explicit-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `f1-binary-x86-explicit-gf2-128-portable-x86-n8` | 8 field-elements | 2281.930 | [1716.977, 2846.883] | 2910.523 | Precise |
| `f1-binary-x86-explicit-gf2-128-pclmul-n8` | 8 field-elements | 154.628 | [154.542, 154.714] | 319.639 | Precise |
| `f1-binary-x86-explicit-gf2-128-vpclmul-n8` | 8 field-elements | 284.574 | [283.980, 285.167] | 308.835 | Precise |
| `f1-binary-x86-explicit-gf2-256-hh-portable-x86-n8` | 8 field-elements | 10629.141 | [10398.844, 10859.438] | 10938.409 | Precise |
| `f1-binary-x86-explicit-gf2-256-hh-pclmul-n8` | 8 field-elements | 717.697 | [712.367, 723.027] | 743.080 | Precise |
| `f1-binary-x86-explicit-gf2-256-hh-vpclmul-n8` | 8 field-elements | 548.529 | [514.420, 582.639] | 596.444 | Precise |
| `f1-binary-x86-explicit-gf2-256-alt-portable-x86-n8` | 8 field-elements | 9028.266 | [7080.312, 10976.219] | 11046.828 | Precise |
| `f1-binary-x86-explicit-gf2-256-alt-portable-x86-n64` | 64 field-elements | 94214.875 | [94102.250, 94327.500] | 95209.413 | Precise |
| `f1-binary-x86-explicit-gf2-256-alt-pclmul-n8` | 8 field-elements | 573.616 | [440.656, 706.576] | 712.103 | Precise |
| `f1-binary-x86-explicit-gf2-256-alt-vpclmul-n64` | 64 field-elements | 5602.008 | [5592.969, 5611.047] | 6354.427 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `f1-binary-x86-explicit-gf2-128-pclmul-n8` | `f1-binary-x86-explicit-gf2-128-portable-x86-n8` | 0.0722 | [0.0543, 0.0900] |
| `f1-binary-x86-explicit-gf2-128-vpclmul-n8` | `f1-binary-x86-explicit-gf2-128-portable-x86-n8` | 0.1328 | [0.1002, 0.1654] |
| `f1-binary-x86-explicit-gf2-256-hh-pclmul-n8` | `f1-binary-x86-explicit-gf2-256-hh-portable-x86-n8` | 0.0675 | [0.0666, 0.0685] |
| `f1-binary-x86-explicit-gf2-256-hh-vpclmul-n8` | `f1-binary-x86-explicit-gf2-256-hh-portable-x86-n8` | 0.0516 | [0.0495, 0.0537] |
| `f1-binary-x86-explicit-gf2-256-alt-pclmul-n8` | `f1-binary-x86-explicit-gf2-256-alt-portable-x86-n8` | 0.0633 | [0.0622, 0.0644] |
| `f1-binary-x86-explicit-gf2-256-alt-vpclmul-n64` | `f1-binary-x86-explicit-gf2-256-alt-portable-x86-n64` | 0.0595 | [0.0593, 0.0596] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
