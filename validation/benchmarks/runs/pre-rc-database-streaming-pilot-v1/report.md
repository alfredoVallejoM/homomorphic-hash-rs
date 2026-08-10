# Benchmark pre-rc-database-streaming-pilot-v1

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `db-r-s65536-m4096-sc` | 4096 row-mutations | 163858733.000 | [163271658.000, 170935451.000] | 174039955.400 | Precise |
| `db-b-s65536-m4096-sc` | 4096 row-mutations | 20711878.000 | [20633050.000, 21330815.000] | 21917391.600 | Precise |
| `db-a-s65536-m4096-sc` | 4096 row-mutations | 20448695.000 | [20338902.000, 20982935.000] | 21417330.800 | Precise |
| `db-r-s65536-m65536` | 65536 row-mutations | 167854865.000 | [165994328.000, 169144690.000] | 173897390.400 | Precise |
| `db-b-s65536-m65536` | 65536 row-mutations | 343099256.000 | [340785743.000, 344157771.000] | 358863145.200 | Precise |
| `db-a-s65536-m65536` | 65536 row-mutations | 258458310.000 | [257822971.000, 266841026.000] | 282801045.800 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `db-b-s65536-m4096-sc` | `db-r-s65536-m4096-sc` | 0.1268 | [0.1207, 0.1302] |
| `db-a-s65536-m4096-sc` | `db-r-s65536-m4096-sc` | 0.1241 | [0.1228, 0.1254] |
| `db-b-s65536-m65536` | `db-r-s65536-m65536` | 2.0422 | [2.0174, 2.0733] |
| `db-a-s65536-m65536` | `db-r-s65536-m65536` | 1.5493 | [1.5280, 1.5883] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
