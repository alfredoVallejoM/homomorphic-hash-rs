# Benchmark pre-rc-comprehensive-smoke-v1

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `field-gf2-128-mul` | 1 field-operation | 291.136 | [255.478, 326.795] | 331.819 | Precise |
| `field-gf2-256-hh-mul` | 1 field-operation | 1276.080 | [1190.750, 1361.410] | 1373.863 | Precise |
| `field-gf2-256-alt-mul` | 1 field-operation | 1188.873 | [1010.719, 1367.027] | 1381.766 | Precise |
| `field-fp251-mul` | 1 field-operation | 5.773 | [5.745, 5.801] | 6.826 | Precise |
| `field-goldilocks-mul` | 1 field-operation | 14.490 | [12.063, 16.918] | 17.051 | Precise |
| `field-scalar-64` | 64 field-elements | 18903.562 | [12601.250, 25205.875] | 25715.541 | Precise |
| `field-detected-64` | 64 field-elements | 479.284 | [476.512, 482.056] | 489.314 | Precise |
| `field-scalar-4096` | 4096 field-elements | 906566.000 | [873182.000, 939950.000] | 1073122.800 | Precise |
| `field-detected-4096` | 4096 field-elements | 38354.062 | [29930.750, 46777.375] | 48002.094 | Precise |
| `field-scalar-65536` | 65536 field-elements | 14386522.000 | [13932866.000, 14840178.000] | 15093997.850 | Precise |
| `field-detected-65536` | 65536 field-elements | 465807.500 | [463211.000, 468404.000] | 474379.000 | Precise |
| `sig-add-build-64` | 64 items | 11993.828 | [11815.844, 12171.812] | 12352.223 | Precise |
| `sig-add-compose-64` | 64 items | 6.473 | [5.486, 7.459] | 7.743 | Precise |
| `sig-add-build-4096` | 4096 items | 790835.500 | [768875.000, 812796.000] | 843664.150 | Precise |
| `sig-add-compose-4096` | 4096 items | 7.265 | [7.243, 7.287] | 7.326 | Precise |
| `sig-seq-build-64` | 64 items | 12074.781 | [12068.406, 12081.156] | 12209.541 | Precise |
| `sig-seq-compose-64` | 64 items | 356.013 | [340.174, 371.853] | 383.540 | Precise |
| `sig-seq-build-4096` | 4096 items | 755234.000 | [740730.000, 769738.000] | 771953.150 | Precise |
| `sig-seq-compose-4096` | 4096 items | 391.968 | [387.969, 395.967] | 424.403 | Precise |
| `sig-bidir-build-64` | 64 items | 12192.031 | [12132.125, 12251.938] | 12554.467 | Precise |
| `sig-bidir-compose-64` | 64 items | 8.184 | [7.270, 9.098] | 10.074 | Precise |
| `sig-bidir-build-4096` | 4096 items | 841978.500 | [799268.000, 884689.000] | 887027.800 | Precise |
| `sig-bidir-compose-4096` | 4096 items | 7.199 | [7.173, 7.225] | 7.253 | Precise |
| `sig-set-build-64` | 64 items | 12651.203 | [12375.781, 12926.625] | 13406.972 | Precise |
| `sig-set-compose-64` | 64 items | 7.225 | [5.734, 8.716] | 8.717 | Precise |
| `sig-set-build-4096` | 4096 items | 767300.500 | [766869.000, 767732.000] | 772983.200 | Precise |
| `sig-set-compose-4096` | 4096 items | 8.706 | [8.561, 8.852] | 16.505 | Precise |
| `sig-multi-set-build-64` | 64 items | 12684.281 | [12554.031, 12814.531] | 13046.061 | Precise |
| `sig-multi-set-compose-64` | 64 items | 7.267 | [7.247, 7.287] | 7.396 | Precise |
| `sig-multi-set-build-4096` | 4096 items | 835918.500 | [776173.000, 895664.000] | 896881.300 | Precise |
| `sig-multi-set-compose-4096` | 4096 items | 9.181 | [7.539, 10.822] | 11.303 | Precise |
| `sig-multi-seq-build-64` | 64 items | 12912.500 | [12045.656, 13779.344] | 13894.792 | Precise |
| `sig-multi-seq-compose-64` | 64 items | 657.908 | [654.250, 661.566] | 691.461 | Precise |
| `sig-multi-seq-build-4096` | 4096 items | 750226.000 | [747977.000, 752475.000] | 782214.950 | Precise |
| `sig-multi-seq-compose-4096` | 4096 items | 702.100 | [656.916, 747.283] | 755.991 | Precise |
| `sig-add-build-payload-1024` | 4096 items | 23158302.000 | [23064811.000, 23251793.000] | 23356200.100 | Precise |
| `sig-multi-seq-build-payload-1024` | 4096 items | 22980398.500 | [22968087.000, 22992710.000] | 23008669.150 | Precise |
| `delta-additive-32` | 32 items | 2.924 | [2.525, 3.323] | 3.394 | Precise |
| `delta-additive-4096` | 4096 items | 0.021 | [0.019, 0.022] | 0.026 | Precise |
| `tree-rebuild-16-leaves` | 16 disjoint-edits | 3800636.500 | [3793679.000, 3807594.000] | 3830683.500 | Precise |
| `tree-adaptive-16-leaves` | 16 disjoint-edits | 1028814.000 | [996974.000, 1060654.000] | 1148926.150 | Precise |
| `tree-rebuild-48-leaves` | 48 disjoint-edits | 3808775.000 | [3807254.000, 3810296.000] | 3844789.800 | Precise |
| `tree-adaptive-48-leaves` | 48 disjoint-edits | 2883142.500 | [2877964.000, 2888321.000] | 3376137.250 | Precise |
| `db-rebuild-scattered-4096` | 4096 row-mutations | 168247335.000 | [166740821.000, 169753849.000] | 173716503.150 | Precise |
| `db-adaptive-scattered-4096` | 4096 row-mutations | 21389039.000 | [21379231.000, 21398847.000] | 24290195.400 | Precise |
| `db-rebuild-scattered-49152` | 49152 row-mutations | 169233472.000 | [169000227.000, 169466717.000] | 171208503.900 | Precise |
| `db-adaptive-scattered-49152` | 49152 row-mutations | 257448286.500 | [256433571.000, 258463002.000] | 261696626.550 | Precise |
| `db-rebuild-clustered-4096` | 4096 row-mutations | 168415666.000 | [167150217.000, 169681115.000] | 182945032.000 | Precise |
| `db-adaptive-clustered-4096` | 4096 row-mutations | 17410157.500 | [16641426.000, 18178889.000] | 18855904.000 | Precise |
| `db-rebuild-clustered-49152` | 49152 row-mutations | 173771166.500 | [172217354.000, 175324979.000] | 176511820.600 | Precise |
| `db-adaptive-clustered-49152` | 49152 row-mutations | 194161181.000 | [193954288.000, 194368074.000] | 196899025.900 | Precise |
| `reconcile-4` | 4 symmetric-difference | 45533.875 | [42932.375, 48135.375] | 50441.975 | Precise |
| `reconcile-16` | 16 symmetric-difference | 166971.531 | [166870.375, 167072.688] | 169462.231 | Precise |
| `reconcile-32` | 32 symmetric-difference | 631070.453 | [612083.938, 650056.969] | 664014.584 | Precise |
| `graph-fast-256` | 256 vertices | 616.237 | [506.252, 726.223] | 745.163 | Precise |
| `graph-fast-16384` | 16384 vertices | 509.054 | [507.429, 510.680] | 540.952 | Precise |
| `graph-fast-131072` | 131072 vertices | 503.132 | [502.549, 503.716] | 515.090 | Precise |
| `graph-exact-6` | 6 vertices | 1911.982 | [1911.094, 1912.870] | 2017.519 | Precise |
| `graph-dag-reuse-6` | 6 vertices | 1734.969 | [1725.214, 1744.724] | 1768.972 | Precise |
| `graph-exact-10` | 10 vertices | 2189.391 | [2171.481, 2207.300] | 2242.150 | Precise |
| `graph-dag-reuse-10` | 10 vertices | 1656.491 | [1608.219, 1704.763] | 2679.716 | Precise |
| `graph-exact-14` | 14 vertices | 2016.980 | [1775.058, 2258.902] | 2408.165 | Precise |
| `graph-dag-reuse-14` | 14 vertices | 2012.312 | [1614.750, 2409.875] | 2616.552 | Precise |
| `tool-manifest-parse` | 1 manifest | 27231.250 | [23153.312, 31309.188] | 33277.706 | Precise |
| `tool-manifest-generate` | 1 generated-package | 413766.500 | [406775.000, 420758.000] | 437322.800 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `field-detected-64` | `field-scalar-64` | 0.0285 | [0.0191, 0.0378] |
| `field-detected-4096` | `field-scalar-4096` | 0.0427 | [0.0318, 0.0536] |
| `field-detected-65536` | `field-scalar-65536` | 0.0324 | [0.0312, 0.0336] |
| `sig-add-compose-64` | `sig-add-build-64` | 0.0005 | [0.0005, 0.0006] |
| `sig-add-compose-4096` | `sig-add-build-4096` | 0.0000 | [0.0000, 0.0000] |
| `sig-seq-compose-64` | `sig-seq-build-64` | 0.0295 | [0.0282, 0.0308] |
| `sig-seq-compose-4096` | `sig-seq-build-4096` | 0.0005 | [0.0005, 0.0005] |
| `sig-bidir-compose-64` | `sig-bidir-build-64` | 0.0007 | [0.0006, 0.0007] |
| `sig-bidir-compose-4096` | `sig-bidir-build-4096` | 0.0000 | [0.0000, 0.0000] |
| `sig-set-compose-64` | `sig-set-build-64` | 0.0006 | [0.0004, 0.0007] |
| `sig-set-compose-4096` | `sig-set-build-4096` | 0.0000 | [0.0000, 0.0000] |
| `sig-multi-set-compose-64` | `sig-multi-set-build-64` | 0.0006 | [0.0006, 0.0006] |
| `sig-multi-set-compose-4096` | `sig-multi-set-build-4096` | 0.0000 | [0.0000, 0.0000] |
| `sig-multi-seq-compose-64` | `sig-multi-seq-build-64` | 0.0512 | [0.0475, 0.0549] |
| `sig-multi-seq-compose-4096` | `sig-multi-seq-build-4096` | 0.0009 | [0.0009, 0.0010] |
| `sig-multi-seq-build-payload-1024` | `sig-add-build-payload-1024` | 0.9923 | [0.9889, 0.9958] |
| `tree-adaptive-16-leaves` | `tree-rebuild-16-leaves` | 0.2707 | [0.2628, 0.2786] |
| `tree-adaptive-48-leaves` | `tree-rebuild-48-leaves` | 0.7570 | [0.7553, 0.7586] |
| `db-adaptive-scattered-4096` | `db-rebuild-scattered-4096` | 0.1271 | [0.1261, 0.1282] |
| `db-adaptive-scattered-49152` | `db-rebuild-scattered-49152` | 1.5213 | [1.5132, 1.5294] |
| `db-adaptive-clustered-4096` | `db-rebuild-clustered-4096` | 0.1033 | [0.0996, 0.1071] |
| `db-adaptive-clustered-49152` | `db-rebuild-clustered-49152` | 1.1174 | [1.1063, 1.1286] |
| `graph-dag-reuse-6` | `graph-exact-6` | 0.9074 | [0.9019, 0.9129] |
| `graph-dag-reuse-10` | `graph-exact-10` | 0.7565 | [0.7406, 0.7723] |
| `graph-dag-reuse-14` | `graph-exact-14` | 0.9883 | [0.9097, 1.0668] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
