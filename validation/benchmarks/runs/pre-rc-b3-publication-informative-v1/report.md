# Benchmark pre-rc-b3-publication-informative-v1

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `field-scalar-1` | 1 field-elements | 225.226 | [221.040, 231.671] | 260.618 | Precise |
| `field-detected-1` | 1 field-elements | 27.400 | [27.372, 27.422] | 27.895 | Precise |
| `field-scalar-8` | 8 field-elements | 1614.624 | [1547.422, 1670.816] | 1977.036 | Precise |
| `field-detected-8` | 8 field-elements | 78.881 | [78.837, 78.914] | 79.833 | Precise |
| `field-scalar-64` | 64 field-elements | 12952.051 | [12374.998, 13412.543] | 21131.744 | Precise |
| `field-detected-64` | 64 field-elements | 482.539 | [482.483, 482.687] | 486.914 | Precise |
| `field-scalar-512` | 512 field-elements | 102739.531 | [98760.406, 106714.969] | 161456.375 | Precise |
| `field-detected-512` | 512 field-elements | 3686.209 | [3684.239, 3686.936] | 4423.661 | Precise |
| `field-scalar-4096` | 4096 field-elements | 833143.750 | [811751.500, 861321.875] | 974323.350 | Precise |
| `field-detected-4096` | 4096 field-elements | 31613.680 | [31586.328, 31636.883] | 35076.380 | Precise |
| `field-scalar-65536` | 65536 field-elements | 13214530.000 | [12869242.500, 14113364.250] | 17196483.900 | Precise |
| `field-detected-65536` | 65536 field-elements | 506243.875 | [465334.250, 509290.688] | 583390.675 | Precise |
| `signature-rebuild-8` | 8 items | 1880.526 | [1870.193, 1884.072] | 2023.299 | Precise |
| `signature-merge-8` | 8 items | 5.369 | [5.367, 5.371] | 5.498 | Precise |
| `signature-rebuild-64` | 64 items | 11940.504 | [11847.699, 11962.192] | 12666.637 | Precise |
| `signature-merge-64` | 64 items | 5.770 | [5.768, 5.774] | 9.073 | Precise |
| `signature-rebuild-512` | 512 items | 91718.562 | [91646.500, 92423.438] | 99313.831 | Precise |
| `signature-merge-512` | 512 items | 5.438 | [5.437, 5.440] | 7.024 | Precise |
| `signature-rebuild-4096` | 4096 items | 730725.875 | [729751.625, 736378.000] | 779675.600 | Precise |
| `signature-merge-4096` | 4096 items | 5.909 | [5.905, 5.916] | 5.998 | Precise |
| `signature-rebuild-65536` | 65536 items | 11771326.250 | [11680352.250, 11797791.000] | 12369708.500 | Precise |
| `signature-merge-65536` | 65536 items | 5.439 | [5.438, 5.440] | 6.114 | Precise |
| `file-rebuild-1` | 1 edited-bytes | 3780873.500 | [3780459.388, 3781536.000] | 3806207.000 | Precise |
| `file-incremental-1` | 1 edited-bytes | 65560.625 | [65552.469, 65581.962] | 65982.341 | Precise |
| `file-rebuild-64` | 64 edited-bytes | 3781204.000 | [3780298.000, 3782117.750] | 3876558.750 | Precise |
| `file-incremental-64` | 64 edited-bytes | 129592.594 | [129549.688, 129631.406] | 138018.594 | Precise |
| `file-rebuild-4096` | 4096 edited-bytes | 3781237.500 | [3780770.250, 3781618.750] | 3795357.800 | Precise |
| `file-incremental-4096` | 4096 edited-bytes | 129676.750 | [129653.281, 129705.414] | 131623.975 | Precise |
| `file-rebuild-16384` | 16384 edited-bytes | 3781659.500 | [3780862.000, 3782604.000] | 4020914.900 | Precise |
| `file-incremental-16384` | 16384 edited-bytes | 246152.844 | [246098.500, 246190.719] | 260320.587 | Precise |
| `file-rebuild-65536` | 65536 edited-bytes | 3783336.250 | [3782914.000, 3783968.500] | 3807893.900 | Precise |
| `file-incremental-65536` | 65536 edited-bytes | 954358.375 | [954055.250, 954498.625] | 957442.175 | Precise |
| `file-rebuild-262144` | 262144 edited-bytes | 3788437.250 | [3787780.000, 3789231.750] | 3806191.300 | Precise |
| `file-incremental-262144` | 262144 edited-bytes | 3802247.000 | [3801650.250, 3803708.500] | 3845023.150 | Precise |
| `database-rebuild-1` | 1 row-mutations | 1098342.000 | [1097179.500, 1098878.250] | 1139343.500 | Precise |
| `database-incremental-1` | 1 row-mutations | 46524.250 | [46371.000, 46677.750] | 73237.650 | Precise |
| `database-rebuild-8` | 8 row-mutations | 1099963.750 | [1098391.500, 1100682.750] | 1114213.950 | Precise |
| `database-incremental-8` | 8 row-mutations | 341213.250 | [340943.000, 341775.000] | 547580.450 | Precise |
| `database-rebuild-32` | 32 row-mutations | 1101161.750 | [1100441.500, 1102241.000] | 1121025.000 | Precise |
| `database-incremental-32` | 32 row-mutations | 1230551.500 | [1230065.000, 1231716.750] | 1260610.600 | Precise |
| `database-rebuild-128` | 128 row-mutations | 1107983.750 | [1107273.750, 1109975.750] | 1129930.200 | Precise |
| `database-incremental-128` | 128 row-mutations | 4611910.250 | [4610019.500, 4612871.250] | 4720522.850 | Precise |
| `database-rebuild-512` | 512 row-mutations | 1131094.250 | [1129008.688, 1134426.575] | 1171845.100 | Precise |
| `database-incremental-512` | 512 row-mutations | 18931824.250 | [18911532.750, 18945413.500] | 19957916.400 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `field-detected-1` | `field-scalar-1` | 0.1216 | [0.1185, 0.1240] |
| `field-detected-8` | `field-scalar-8` | 0.0488 | [0.0472, 0.0509] |
| `field-detected-64` | `field-scalar-64` | 0.0373 | [0.0360, 0.0390] |
| `field-detected-512` | `field-scalar-512` | 0.0364 | [0.0346, 0.0373] |
| `field-detected-4096` | `field-scalar-4096` | 0.0374 | [0.0352, 0.0389] |
| `field-detected-65536` | `field-scalar-65536` | 0.0359 | [0.0340, 0.0380] |
| `signature-merge-8` | `signature-rebuild-8` | 0.0029 | [0.0028, 0.0029] |
| `signature-merge-64` | `signature-rebuild-64` | 0.0005 | [0.0005, 0.0005] |
| `signature-merge-512` | `signature-rebuild-512` | 0.0001 | [0.0001, 0.0001] |
| `signature-merge-4096` | `signature-rebuild-4096` | 0.0000 | [0.0000, 0.0000] |
| `signature-merge-65536` | `signature-rebuild-65536` | 0.0000 | [0.0000, 0.0000] |
| `file-incremental-1` | `file-rebuild-1` | 0.0173 | [0.0173, 0.0173] |
| `file-incremental-64` | `file-rebuild-64` | 0.0343 | [0.0343, 0.0343] |
| `file-incremental-4096` | `file-rebuild-4096` | 0.0343 | [0.0343, 0.0343] |
| `file-incremental-16384` | `file-rebuild-16384` | 0.0651 | [0.0651, 0.0651] |
| `file-incremental-65536` | `file-rebuild-65536` | 0.2522 | [0.2522, 0.2523] |
| `file-incremental-262144` | `file-rebuild-262144` | 1.0037 | [1.0034, 1.0040] |
| `database-incremental-1` | `database-rebuild-1` | 0.0423 | [0.0422, 0.0425] |
| `database-incremental-8` | `database-rebuild-8` | 0.3106 | [0.3098, 0.3110] |
| `database-incremental-32` | `database-rebuild-32` | 1.1172 | [1.1164, 1.1184] |
| `database-incremental-128` | `database-rebuild-128` | 4.1602 | [4.1540, 4.1670] |
| `database-incremental-512` | `database-rebuild-512` | 16.7304 | [16.6923, 16.7646] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
