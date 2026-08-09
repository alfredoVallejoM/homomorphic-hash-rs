# Benchmark pre-rc-b3-pilot-scaling-v1

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `field-scalar-1` | 1 field-elements | 225.880 | [220.623, 243.391] | 351.615 | Precise |
| `field-detected-1` | 1 field-elements | 27.411 | [27.298, 27.451] | 67.210 | Precise |
| `field-scalar-8` | 8 field-elements | 1629.404 | [1506.094, 1716.656] | 2351.618 | Precise |
| `field-detected-8` | 8 field-elements | 79.355 | [79.009, 97.651] | 116.144 | Precise |
| `field-scalar-64` | 64 field-elements | 12493.609 | [12224.797, 15298.359] | 20116.964 | Precise |
| `field-detected-64` | 64 field-elements | 483.498 | [482.719, 486.356] | 608.769 | Precise |
| `field-scalar-512` | 512 field-elements | 101200.375 | [93428.438, 123864.000] | 177017.888 | Inconclusive |
| `field-detected-512` | 512 field-elements | 3702.889 | [3686.852, 4566.688] | 5156.803 | Precise |
| `field-scalar-4096` | 4096 field-elements | 836132.000 | [781155.000, 872745.500] | 1030639.150 | Precise |
| `field-detected-4096` | 4096 field-elements | 31662.688 | [30544.364, 37913.656] | 46001.188 | Precise |
| `field-scalar-65536` | 65536 field-elements | 14161072.000 | [13081517.075, 14767187.000] | 16585834.000 | Precise |
| `field-detected-65536` | 65536 field-elements | 492176.500 | [466878.500, 513112.500] | 570337.350 | Precise |
| `signature-rebuild-8` | 8 items | 1871.371 | [1869.701, 1885.264] | 2019.586 | Precise |
| `signature-merge-8` | 8 items | 5.467 | [5.446, 6.365] | 7.248 | Precise |
| `signature-rebuild-64` | 64 items | 11976.617 | [11913.047, 12262.734] | 12613.212 | Precise |
| `signature-merge-64` | 64 items | 5.926 | [5.919, 8.182] | 8.259 | Inconclusive |
| `signature-rebuild-512` | 512 items | 92054.125 | [91585.000, 94103.000] | 97262.581 | Precise |
| `signature-merge-512` | 512 items | 5.451 | [5.444, 5.474] | 7.043 | Precise |
| `signature-rebuild-4096` | 4096 items | 735298.000 | [730698.000, 739433.000] | 784799.750 | Precise |
| `signature-merge-4096` | 4096 items | 5.449 | [5.442, 5.457] | 7.404 | Precise |
| `signature-rebuild-65536` | 65536 items | 11776942.500 | [11714689.000, 11880494.000] | 12373958.550 | Precise |
| `signature-merge-65536` | 65536 items | 5.375 | [5.370, 5.386] | 5.528 | Precise |
| `file-rebuild-1` | 1 edited-bytes | 3786597.000 | [3785192.138, 3790902.000] | 3953399.100 | Precise |
| `file-incremental-1` | 1 edited-bytes | 65757.062 | [65614.125, 66029.375] | 70113.262 | Precise |
| `file-rebuild-64` | 64 edited-bytes | 3785470.000 | [3783935.500, 3787439.500] | 3870782.650 | Precise |
| `file-incremental-64` | 64 edited-bytes | 130491.500 | [129994.250, 130798.375] | 469775.325 | Precise |
| `file-rebuild-4096` | 4096 edited-bytes | 3785310.500 | [3784404.450, 3790671.000] | 3914017.550 | Precise |
| `file-incremental-4096` | 4096 edited-bytes | 130341.375 | [130060.250, 130597.250] | 137607.925 | Precise |
| `file-rebuild-16384` | 16384 edited-bytes | 3786324.000 | [3784626.000, 3790616.500] | 3883569.550 | Precise |
| `file-incremental-16384` | 16384 edited-bytes | 246688.250 | [246398.750, 246974.500] | 256860.862 | Precise |
| `file-rebuild-65536` | 65536 edited-bytes | 3791015.000 | [3788180.500, 3799578.500] | 3908411.000 | Precise |
| `file-incremental-65536` | 65536 edited-bytes | 955792.000 | [955576.500, 957435.500] | 1000595.650 | Precise |
| `file-rebuild-262144` | 262144 edited-bytes | 3791918.500 | [3791455.000, 3800199.500] | 3955544.450 | Precise |
| `file-incremental-262144` | 262144 edited-bytes | 3804410.500 | [3802663.000, 3815729.862] | 3870860.750 | Precise |
| `database-rebuild-1` | 1 row-mutations | 1100833.000 | [1100197.000, 1102779.000] | 1144992.400 | Precise |
| `database-incremental-1` | 1 row-mutations | 45552.000 | [45232.000, 45878.838] | 50565.050 | Precise |
| `database-rebuild-8` | 8 row-mutations | 1106143.000 | [1099650.000, 1114510.000] | 1228609.700 | Precise |
| `database-incremental-8` | 8 row-mutations | 344360.000 | [342727.225, 348224.000] | 540720.450 | Precise |
| `database-rebuild-32` | 32 row-mutations | 1102796.000 | [1100576.000, 1108073.500] | 1165798.250 | Precise |
| `database-incremental-32` | 32 row-mutations | 1232705.500 | [1229327.500, 1236966.000] | 1301304.150 | Precise |
| `database-rebuild-128` | 128 row-mutations | 1110432.000 | [1107939.500, 1117999.000] | 1142901.650 | Precise |
| `database-incremental-128` | 128 row-mutations | 4626843.000 | [4615307.500, 4670366.000] | 4855623.700 | Precise |
| `database-rebuild-512` | 512 row-mutations | 1132043.500 | [1129333.000, 1137083.000] | 1219809.250 | Precise |
| `database-incremental-512` | 512 row-mutations | 19029170.500 | [18884480.000, 19088610.587] | 19572246.000 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `field-detected-1` | `field-scalar-1` | 0.1210 | [0.1123, 0.1265] |
| `field-detected-8` | `field-scalar-8` | 0.0510 | [0.0473, 0.0583] |
| `field-detected-64` | `field-scalar-64` | 0.0388 | [0.0326, 0.0396] |
| `field-detected-512` | `field-scalar-512` | 0.0367 | [0.0298, 0.0462] |
| `field-detected-4096` | `field-scalar-4096` | 0.0376 | [0.0355, 0.0447] |
| `field-detected-65536` | `field-scalar-65536` | 0.0351 | [0.0324, 0.0384] |
| `signature-merge-8` | `signature-rebuild-8` | 0.0029 | [0.0029, 0.0034] |
| `signature-merge-64` | `signature-rebuild-64` | 0.0005 | [0.0005, 0.0007] |
| `signature-merge-512` | `signature-rebuild-512` | 0.0001 | [0.0001, 0.0001] |
| `signature-merge-4096` | `signature-rebuild-4096` | 0.0000 | [0.0000, 0.0000] |
| `signature-merge-65536` | `signature-rebuild-65536` | 0.0000 | [0.0000, 0.0000] |
| `file-incremental-1` | `file-rebuild-1` | 0.0174 | [0.0173, 0.0174] |
| `file-incremental-64` | `file-rebuild-64` | 0.0345 | [0.0343, 0.0345] |
| `file-incremental-4096` | `file-rebuild-4096` | 0.0344 | [0.0343, 0.0345] |
| `file-incremental-16384` | `file-rebuild-16384` | 0.0651 | [0.0650, 0.0652] |
| `file-incremental-65536` | `file-rebuild-65536` | 0.2521 | [0.2516, 0.2525] |
| `file-incremental-262144` | `file-rebuild-262144` | 1.0030 | [1.0010, 1.0046] |
| `database-incremental-1` | `database-rebuild-1` | 0.0413 | [0.0411, 0.0416] |
| `database-incremental-8` | `database-rebuild-8` | 0.3109 | [0.3079, 0.3133] |
| `database-incremental-32` | `database-rebuild-32` | 1.1191 | [1.1092, 1.1228] |
| `database-incremental-128` | `database-rebuild-128` | 4.1543 | [4.1350, 4.1953] |
| `database-incremental-512` | `database-rebuild-512` | 16.7652 | [16.6530, 16.8876] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
