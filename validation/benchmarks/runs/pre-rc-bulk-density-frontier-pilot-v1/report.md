# Benchmark pre-rc-bulk-density-frontier-pilot-v1

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `st-r-s64m-e12288-sc` | 12288 disjoint-edits | 1008503745.000 | [1007953101.000, 1021500853.000] | 1043874038.800 | Precise |
| `st-b-s64m-e12288-sc` | 12288 disjoint-edits | 752205630.000 | [751857215.000, 752851342.000] | 753693723.400 | Precise |
| `st-a-s64m-e12288-sc` | 12288 disjoint-edits | 752622890.000 | [751942534.000, 753792206.000] | 754280187.400 | Precise |
| `st-r-s64m-e12288-cl` | 12288 disjoint-edits | 1007741194.000 | [1007291141.000, 1008474706.000] | 1009647605.400 | Precise |
| `st-b-s64m-e12288-cl` | 12288 disjoint-edits | 746469459.000 | [745578967.000, 746857261.000] | 747731177.200 | Precise |
| `st-a-s64m-e12288-cl` | 12288 disjoint-edits | 746258753.000 | [745957271.000, 746499850.000] | 751121110.000 | Precise |
| `st-r-s64m-e16384` | 16384 disjoint-edits | 1008494595.000 | [1008006694.000, 1008642905.000] | 1011024373.200 | Precise |
| `st-b-s64m-e16384` | 16384 disjoint-edits | 994835683.000 | [994804315.000, 995499304.000] | 996789935.000 | Precise |
| `st-a-s64m-e16384` | 16384 disjoint-edits | 991735251.000 | [990677277.000, 992112916.000] | 992689733.600 | Precise |
| `db-r-s65536-m49152-sc` | 49152 row-mutations | 168131025.000 | [166751267.000, 173707095.000] | 174565296.600 | Precise |
| `db-b-s65536-m49152-sc` | 49152 row-mutations | 267070682.000 | [266785083.000, 268906232.000] | 276730237.400 | Precise |
| `db-a-s65536-m49152-sc` | 49152 row-mutations | 260543950.000 | [259231587.000, 265090554.000] | 267836706.600 | Precise |
| `db-r-s65536-m49152-cl` | 49152 row-mutations | 170774396.000 | [169125119.000, 173113065.000] | 174262906.000 | Precise |
| `db-b-s65536-m49152-cl` | 49152 row-mutations | 259151788.000 | [258205093.000, 260015852.000] | 269311655.600 | Precise |
| `db-a-s65536-m49152-cl` | 49152 row-mutations | 201626870.000 | [200297015.000, 206898187.000] | 211174622.800 | Precise |
| `db-r-s65536-m65536` | 65536 row-mutations | 167807841.000 | [167495968.000, 170703855.000] | 171372725.000 | Precise |
| `db-b-s65536-m65536` | 65536 row-mutations | 358084321.000 | [356703706.000, 360246924.000] | 366281512.800 | Precise |
| `db-a-s65536-m65536` | 65536 row-mutations | 272236988.000 | [271546648.000, 273721215.000] | 287131398.600 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `st-b-s64m-e12288-sc` | `st-r-s64m-e12288-sc` | 0.7459 | [0.7361, 0.7469] |
| `st-a-s64m-e12288-sc` | `st-r-s64m-e12288-sc` | 0.7465 | [0.7361, 0.7474] |
| `st-b-s64m-e12288-cl` | `st-r-s64m-e12288-cl` | 0.7403 | [0.7402, 0.7412] |
| `st-a-s64m-e12288-cl` | `st-r-s64m-e12288-cl` | 0.7405 | [0.7401, 0.7408] |
| `st-b-s64m-e16384` | `st-r-s64m-e16384` | 0.9866 | [0.9863, 0.9876] |
| `st-a-s64m-e16384` | `st-r-s64m-e16384` | 0.9833 | [0.9828, 0.9839] |
| `db-b-s65536-m49152-sc` | `db-r-s65536-m49152-sc` | 1.5978 | [1.5366, 1.6016] |
| `db-a-s65536-m49152-sc` | `db-r-s65536-m49152-sc` | 1.5496 | [1.4968, 1.5897] |
| `db-b-s65536-m49152-cl` | `db-r-s65536-m49152-cl` | 1.5122 | [1.4949, 1.5323] |
| `db-a-s65536-m49152-cl` | `db-r-s65536-m49152-cl` | 1.1843 | [1.1647, 1.2056] |
| `db-b-s65536-m65536` | `db-r-s65536-m65536` | 2.1278 | [2.0977, 2.1468] |
| `db-a-s65536-m65536` | `db-r-s65536-m65536` | 1.6233 | [1.5907, 1.6328] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
