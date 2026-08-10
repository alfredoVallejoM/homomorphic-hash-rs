# Benchmark pre-rc-comprehensive-pilot-v1

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `set-k1-build-64` | 64 items | 12263.320 | [12160.398, 12466.523] | 14122.503 | Precise |
| `set-k1-compose-64` | 64 items | 6.486 | [6.444, 11.769] | 17.120 | Inconclusive |
| `set-k2-build-64` | 64 items | 12597.648 | [12487.719, 13218.141] | 14285.022 | Precise |
| `set-k2-compose-64` | 64 items | 7.456 | [7.444, 7.749] | 11.864 | Precise |
| `set-k3-build-64` | 64 items | 12634.977 | [12575.695, 12669.391] | 13651.455 | Precise |
| `set-k3-compose-64` | 64 items | 8.704 | [8.698, 12.722] | 16.823 | Inconclusive |
| `set-k4-build-64` | 64 items | 14660.742 | [14538.664, 14679.859] | 15328.623 | Precise |
| `set-k4-compose-64` | 64 items | 11.008 | [10.981, 11.033] | 12.258 | Precise |
| `seq-k1-build-4096` | 4096 items | 749940.000 | [739392.000, 774313.150] | 986954.150 | Precise |
| `seq-k1-compose-4096` | 4096 items | 334.946 | [334.258, 335.719] | 349.303 | Precise |
| `seq-k2-build-4096` | 4096 items | 747799.500 | [741655.000, 749883.000] | 836533.050 | Precise |
| `seq-k2-compose-4096` | 4096 items | 653.864 | [653.013, 700.457] | 757.787 | Precise |
| `seq-k3-build-4096` | 4096 items | 752446.500 | [744287.500, 784344.000] | 880062.250 | Precise |
| `seq-k3-compose-4096` | 4096 items | 987.494 | [985.061, 1118.469] | 1131.486 | Precise |
| `seq-k4-build-4096` | 4096 items | 787592.000 | [779681.000, 794840.000] | 821093.750 | Precise |
| `seq-k4-compose-4096` | 4096 items | 1296.190 | [1294.390, 1388.080] | 1510.961 | Precise |
| `set-k4-payload-8` | 4096 items | 734138.000 | [731170.000, 780626.650] | 888229.350 | Precise |
| `set-k4-payload-64` | 4096 items | 1924683.500 | [1921573.500, 1961855.000] | 2028140.950 | Precise |
| `set-k4-payload-1024` | 4096 items | 23090709.500 | [23027901.000, 23158736.000] | 23440111.300 | Precise |
| `seq-k4-payload-8` | 4096 items | 612970.500 | [607260.000, 625617.000] | 660252.400 | Precise |
| `seq-k4-payload-64` | 4096 items | 1805965.500 | [1803317.000, 1852797.000] | 1912513.250 | Precise |
| `seq-k4-payload-1024` | 4096 items | 22971974.000 | [22934608.000, 23023996.000] | 23188723.600 | Precise |
| `graph-cycle-4096` | 4096 vertices | 501.473 | [499.493, 511.929] | 751.573 | Precise |
| `graph-regular8-4096` | 4096 vertices | 1131.238 | [1121.868, 1165.542] | 1686.023 | Precise |
| `graph-star-4096` | 4096 vertices | 491.878 | [491.320, 493.233] | 501.422 | Precise |
| `graph-cycle-131072` | 131072 vertices | 501.857 | [501.048, 505.357] | 542.719 | Precise |
| `graph-regular8-131072` | 131072 vertices | 1150.370 | [1143.887, 1150.947] | 1190.253 | Precise |
| `graph-star-131072` | 131072 vertices | 496.898 | [494.565, 511.196] | 554.150 | Precise |
| `graph-full-edit-1024` | 1024 vertices | 1564097.000 | [1543061.000, 1590583.112] | 2064347.550 | Precise |
| `graph-incremental-edit-1024` | 1024 vertices | 545238.500 | [543066.000, 548126.000] | 590960.750 | Precise |
| `graph-full-edit-16384` | 16384 vertices | 24606070.500 | [24455321.000, 24702868.000] | 25641573.050 | Precise |
| `graph-incremental-edit-16384` | 16384 vertices | 8335341.500 | [8294043.000, 8430766.500] | 8718568.750 | Precise |
| `graph-full-edit-131072` | 131072 vertices | 196143915.000 | [195193935.500, 197251858.000] | 209328895.900 | Precise |
| `graph-incremental-edit-131072` | 131072 vertices | 70586845.000 | [68690620.000, 72427352.500] | 93946400.450 | Precise |
| `graph-exact-path-8` | 8 vertices | 1818.304 | [1813.275, 1830.126] | 2091.895 | Precise |
| `graph-exact-path-16` | 16 vertices | 1718.980 | [1710.566, 1728.083] | 1802.158 | Precise |
| `graph-exact-cycle-8` | 8 vertices | 5409.695 | [5322.750, 6212.469] | 6640.969 | Precise |
| `graph-exact-cycle-12` | 12 vertices | 6263.875 | [6208.052, 7470.667] | 7739.288 | Inconclusive |
| `graph-exact-cycle-16` | 16 vertices | 7206.676 | [7166.188, 7616.398] | 9395.244 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `set-k1-compose-64` | `set-k1-build-64` | 0.0005 | [0.0005, 0.0010] |
| `set-k2-compose-64` | `set-k2-build-64` | 0.0006 | [0.0006, 0.0006] |
| `set-k3-compose-64` | `set-k3-build-64` | 0.0007 | [0.0007, 0.0010] |
| `set-k4-compose-64` | `set-k4-build-64` | 0.0008 | [0.0008, 0.0008] |
| `seq-k1-compose-4096` | `seq-k1-build-4096` | 0.0004 | [0.0004, 0.0005] |
| `seq-k2-compose-4096` | `seq-k2-build-4096` | 0.0009 | [0.0009, 0.0009] |
| `seq-k3-compose-4096` | `seq-k3-build-4096` | 0.0013 | [0.0013, 0.0014] |
| `seq-k4-compose-4096` | `seq-k4-build-4096` | 0.0016 | [0.0016, 0.0018] |
| `graph-incremental-edit-1024` | `graph-full-edit-1024` | 0.3496 | [0.3403, 0.3543] |
| `graph-incremental-edit-16384` | `graph-full-edit-16384` | 0.3381 | [0.3373, 0.3463] |
| `graph-incremental-edit-131072` | `graph-full-edit-131072` | 0.3588 | [0.3498, 0.3702] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
