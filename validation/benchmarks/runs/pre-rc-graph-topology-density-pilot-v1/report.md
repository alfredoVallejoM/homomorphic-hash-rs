# Benchmark pre-rc-graph-topology-density-pilot-v1

Clasificación: `Informative`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `graph-mesh-4096` | 4096 vertices | 680.134 | [678.966, 696.274] | 748.418 | Precise |
| `graph-regular8-4096` | 4096 vertices | 1257.134 | [1253.397, 1265.631] | 1279.587 | Precise |
| `graph-regular32-4096` | 4096 vertices | 3341.311 | [3328.910, 3389.810] | 3513.935 | Precise |
| `graph-mesh-16384` | 16384 vertices | 679.996 | [677.920, 690.597] | 697.498 | Precise |
| `graph-regular8-16384` | 16384 vertices | 1263.903 | [1261.585, 1266.953] | 1273.586 | Precise |
| `graph-regular32-16384` | 16384 vertices | 3338.790 | [3338.167, 3416.232] | 3681.843 | Precise |
| `graph-full-topology-1024` | 1024 vertices | 1616862.000 | [1598407.500, 1633026.500] | 1670053.850 | Precise |
| `graph-incremental-topology-1024` | 1024 vertices | 656446.000 | [647714.000, 685224.500] | 692685.800 | Precise |
| `graph-full-topology-16384` | 16384 vertices | 25870937.000 | [25740382.000, 25997862.000] | 26402296.850 | Precise |
| `graph-incremental-topology-16384` | 16384 vertices | 9259522.500 | [9183602.000, 9662142.000] | 9748644.750 | Precise |

## Comparaciones pareadas

| Celda | Baseline | Ratio mediano | IC 95 % |
|---|---|---:|---:|
| `graph-incremental-topology-1024` | `graph-full-topology-1024` | 0.4078 | [0.3976, 0.4238] |
| `graph-incremental-topology-16384` | `graph-full-topology-16384` | 0.3597 | [0.3536, 0.3737] |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
