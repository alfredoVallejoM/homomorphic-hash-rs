# Benchmark algesum-c3-t1-r1-d1-v1-d1-database-state-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `d1-database-state-row-wire-d4096-p16-n1` | 1 rows-or-mutations | 442.241 | [353.232, 531.250] | 539.447 | Precise |
| `d1-database-state-mixed-d4096-p4096-n1` | 1 rows-or-mutations | 546712.500 | [537599.000, 555826.000] | 618471.600 | Precise |
| `d1-database-state-checkpoint-replay-d4096-p4096-n8` | 8 rows-or-mutations | 11760.328 | [10000.000, 13520.656] | 13716.366 | Precise |
| `d1-database-state-schema-narrow-d4096-p4096-n64` | 64 rows-or-mutations | 392.586 | [392.053, 393.120] | 398.974 | Precise |
| `d1-database-state-schema-medium-d65536-p4096-n1` | 1 rows-or-mutations | 23621.531 | [20980.500, 26262.562] | 27475.653 | Precise |
| `d1-database-state-schema-wide-d65536-p4096-n8` | 8 rows-or-mutations | 137587.000 | [137488.500, 137685.500] | 141911.800 | Precise |
| `d1-database-state-route-d65536-p4096-n64` | 64 rows-or-mutations | 14117751.000 | [12867692.000, 15367810.000] | 16501777.150 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
