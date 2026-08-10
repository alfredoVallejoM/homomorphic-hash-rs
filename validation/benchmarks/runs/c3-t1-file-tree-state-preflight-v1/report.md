# Benchmark algesum-c3-t1-r1-d1-v1-t1-file-tree-state-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `t1-file-tree-state-chunk-build-p512-n1` | 1 bytes | 5642.805 | [5438.688, 5846.922] | 9231.330 | Precise |
| `t1-file-tree-state-chunk-wire-p65536-n1` | 1 bytes | 36.779 | [36.270, 37.288] | 37.340 | Precise |
| `t1-file-tree-state-checkpoint-p4096-n4096` | 4096 bytes | 2.240 | [2.209, 2.270] | 2.354 | Precise |
| `t1-file-tree-state-restore-p512-n1048576` | 1048576 bytes | 39.743 | [25.724, 53.761] | 68.646 | Inconclusive |
| `t1-file-tree-state-grow-shrink-p1048576-n1048576` | 1048576 bytes | 27858445.000 | [27732663.000, 27984227.000] | 28011993.400 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
