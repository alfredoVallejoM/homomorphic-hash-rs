# Benchmark algesum-c3-f3-s3-v1-s1-base-signatures-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `s1-base-signatures-remove-multiset-p1-n1` | 1 items | 244.069 | [242.871, 245.268] | 250.487 | Precise |
| `s1-base-signatures-remove-sequence-p1-n4` | 4 items | 474.853 | [474.497, 475.208] | 506.091 | Precise |
| `s1-base-signatures-wire-additive-p1-n16` | 16 items | 675.254 | [671.189, 679.318] | 717.618 | Precise |
| `s1-base-signatures-wire-sequence-p1-n256` | 256 items | 662.901 | [650.068, 675.734] | 812.759 | Precise |
| `s1-base-signatures-wire-bidirectional-p16-n3` | 3 items | 968.219 | [929.727, 1006.711] | 1046.611 | Precise |
| `s1-base-signatures-wire-multiset-p16-n15` | 15 items | 641.294 | [619.959, 662.629] | 669.908 | Precise |
| `s1-base-signatures-restore-additive-p16-n64` | 64 items | 397.234 | [355.295, 439.173] | 955.522 | Precise |
| `s1-base-signatures-restore-sequence-p256-n2` | 2 items | 388.256 | [356.594, 419.919] | 422.460 | Precise |
| `s1-base-signatures-restore-bidirectional-p256-n8` | 8 items | 705.590 | [667.779, 743.400] | 758.774 | Precise |
| `s1-base-signatures-restore-multiset-p256-n32` | 32 items | 379.539 | [378.958, 380.119] | 395.988 | Precise |
| `s1-base-signatures-pattern-distinct-p4096-n1` | 1 items | 22520.031 | [22360.250, 22679.812] | 27655.303 | Precise |
| `s1-base-signatures-pattern-repeated-p4096-n7` | 7 items | 154034.500 | [152495.500, 155573.500] | 158590.825 | Precise |
| `s1-base-signatures-pattern-empty-p4096-n31` | 31 items | 3314.293 | [3156.047, 3472.539] | 3521.752 | Precise |
| `s1-base-signatures-pattern-skewed-p4096-n1024` | 1024 items | 1484662.500 | [1484521.000, 1484804.000] | 1490533.100 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
