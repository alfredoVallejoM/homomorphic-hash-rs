# Benchmark algesum-c3-x2-v1-x2-wire-roundtrip-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `x2-wire-roundtrip-additive-p1-n1` | 1 items-or-bytes | 690.209 | [670.123, 710.295] | 965.485 | Precise |
| `x2-wire-roundtrip-sequence-p1-n1` | 1 items-or-bytes | 648.901 | [620.553, 677.250] | 691.910 | Precise |
| `x2-wire-roundtrip-bidirectional-p64-n1` | 1 items-or-bytes | 985.605 | [934.512, 1036.699] | 1899.529 | Precise |
| `x2-wire-roundtrip-multiset-p64-n1` | 1 items-or-bytes | 653.471 | [642.123, 664.818] | 681.175 | Precise |
| `x2-wire-roundtrip-multi-sequence-p4096-n1` | 1 items-or-bytes | 462.075 | [430.612, 493.538] | 708.553 | Precise |
| `x2-wire-roundtrip-multi-multiset-p4096-n1` | 1 items-or-bytes | 459.167 | [457.823, 460.510] | 463.804 | Precise |
| `x2-wire-roundtrip-summary-tree-p1-n64` | 64 items-or-bytes | 7046.961 | [6625.734, 7468.188] | 8007.592 | Precise |
| `x2-wire-roundtrip-database-row-p1-n64` | 64 items-or-bytes | 521.758 | [518.140, 525.377] | 983.713 | Precise |
| `x2-wire-roundtrip-reconciliation-p64-n64` | 64 items-or-bytes | 634.446 | [531.438, 737.455] | 741.678 | Precise |
| `x2-wire-roundtrip-dag-p64-n64` | 64 items-or-bytes | 41.328 | [41.125, 41.531] | 44.109 | Precise |
| `x2-wire-roundtrip-journal-p4096-n64` | 64 items-or-bytes | 196006.750 | [180537.000, 211476.500] | 227639.850 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
