# Benchmark algesum-c3-f3-s3-v1-s3-state-delta-journal-preflight

Clasificación: `Smoke`. Claims publicables: `false`. Las firmas medidas son resúmenes algebraicos no criptográficos.

| Celda | Escala | Mediana ns/unidad | IC 95 % | p95 | Estado |
|---|---:|---:|---:|---:|---|
| `s3-state-delta-journal-tracked-sequence-update-p1-n1` | 1 items | 300.981 | [270.325, 331.638] | 346.942 | Precise |
| `s3-state-delta-journal-tracked-multiset-update-p1-n3` | 3 items | 465.856 | [464.764, 466.949] | 468.929 | Precise |
| `s3-state-delta-journal-compact-additive-p1-n8` | 8 items | 340.500 | [313.694, 367.305] | 375.462 | Precise |
| `s3-state-delta-journal-compact-sequence-p1-n16` | 16 items | 263.149 | [260.591, 265.708] | 330.194 | Precise |
| `s3-state-delta-journal-compact-bidirectional-p1-n64` | 64 items | 316.414 | [262.997, 369.830] | 374.293 | Precise |
| `s3-state-delta-journal-compact-multiset-p16-n1` | 1 items | 334.851 | [277.191, 392.511] | 395.837 | Precise |
| `s3-state-delta-journal-tracked-sequence-snapshot-p16-n4` | 4 items | 2843.016 | [2810.945, 2875.086] | 2904.336 | Precise |
| `s3-state-delta-journal-tracked-multiset-snapshot-p16-n8` | 8 items | 4716.258 | [4319.000, 5113.516] | 5185.873 | Precise |
| `s3-state-delta-journal-delta-additive-p16-n31` | 31 items | 87.318 | [82.961, 91.676] | 97.529 | Precise |
| `s3-state-delta-journal-delta-multiset-p16-n64` | 64 items | 150.189 | [138.231, 162.146] | 165.524 | Precise |
| `s3-state-delta-journal-delta-sequence-append-p256-n2` | 2 items | 476.042 | [467.805, 484.278] | 499.981 | Precise |
| `s3-state-delta-journal-delta-sequence-trim-p256-n4` | 4 items | 461.244 | [447.920, 474.567] | 535.192 | Precise |
| `s3-state-delta-journal-journal-append-replay-p256-n15` | 15 items | 44035.562 | [40939.250, 47131.875] | 50006.231 | Precise |
| `s3-state-delta-journal-journal-truncated-p256-n31` | 31 items | 98961.125 | [85999.750, 111922.500] | 128698.075 | Precise |
| `s3-state-delta-journal-journal-corrupt-p256-n256` | 256 items | 374277.000 | [372945.000, 375609.000] | 381734.400 | Precise |

Los datos crudos, orden de ejecución, entorno y checksums acompañan este informe. `Smoke` e `Informative` no autorizan claims principales.
