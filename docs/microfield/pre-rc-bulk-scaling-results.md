# Escalado de lotes en summary tree y base de datos

Fecha: 10 de agosto de 2026.

Estado: implementación y evidencia `Informative` cerradas en la rama
`rc/rc7-correctness`; pendiente de réplica `Controlled` antes de formular
claims públicos.

## Alcance y veredicto

Este trabajo mejora exclusivamente las dos rutas que limitaban los lotes
grandes:

1. reemplazos disjuntos en `HomomorphicSummaryTree`;
2. transacciones multi-fila en `PartitionedDatabase`.

No existe una barrera general en 256 ediciones. El punto de equilibrio depende
de la **fracción** de hojas o filas modificadas, su distribución entre
particiones y el coste fijo del dataset. En el host informativo actual:

- el árbol de 64 MiB conserva ventaja frente a rebuild con 1.024, 8.192 y
  12.288 hojas tocadas (6,25 %, 50 % y 75 %); solo llega aproximadamente a
  paridad cuando cambia el 100 % de sus 16.384 hojas;
- la DB de 65.536 filas aplica 4.096 updates dispersos en 20,45–20,71 ms,
  frente a 163,86 ms del rebuild seleccionado;
- una transacción que cambia todas las 65.536 filas sigue favoreciendo el
  rebuild: 167,85 ms frente a 258,46 ms por la ruta adaptativa y 343,10 ms por
  bulk puro.

Las firmas y raíces medidas son resúmenes algebraicos homomórficos **no
criptográficos**. El SHA-256 usado por `TransactionId` es únicamente el
identificador de contenido del wire transaccional; no convierte la firma
homomórfica en un mecanismo de autenticación.

## Cambios implementados

### Árbol de resúmenes

- `SummaryRangeEdit` representa un reemplazo poseído y con longitud estable.
- `replace_ranges_with_policy` valida todo el lote antes de modificar estado,
  ordena rangos, rechaza solapamientos y cambios de longitud, clona cada hoja
  afectada una vez y recalcula cada ancestro compartido una vez.
- `SummaryEditPath::BulkLocalTree` hace observable la ruta elegida.
- `SummaryEditPolicy::adaptive(bytes, numerator, denominator)` combina un
  techo absoluto con una fracción máxima de hojas tocadas.
- La política por defecto no impone un umbral universal: mantiene el bulk
  local para reemplazos de longitud fija. La aplicación puede fijar su umbral
  con mediciones propias.

### Base de datos

- Las mutaciones se agrupan por partición. Cada grupo prepara una firma de
  filas retiradas y otra de filas añadidas, y aplica un solo delta algebraico
  por partición.
- La preparación y validación son de dos fases: las particiones dispersas ya
  no se clonan completas; solo se materializa un candidato completo para una
  partición que la política clasifique como densa.
- `DatabaseApplyPolicy::adaptive` decide por densidad de cada partición y puede
  producir `Incremental`, `PartitionHybrid` o `PartitionRebuild`. El informe
  expone `rebuilt_partitions`.
- `TransactionDelta` conserva claves primarias y longitud canónica calculadas
  una vez. `TransactionId` se deriva en streaming sobre exactamente el mismo
  framing `MFTX`; `to_canonical_bytes` y el wire persistido no cambian.
- La ruta de rebuild autoritativo continúa verificando que las filas externas
  sean exactamente el resultado de todas las before/after images antes de
  publicar una revisión.

Checkpoints funcionales: `6b95fce`, `d7f33e8` y `73a14cc`.

## Corrección ampliada

Además de la suite previa se añadieron:

- coalescencia de hojas y ancestros compartidos frente a la aplicación
  secuencial;
- selección adaptativa y rechazo atómico de lotes solapados o inválidos;
- 200 revisiones bulk aleatorias, comparando después de cada commit bytes,
  raíz, revisión y rebuild exacto;
- política híbrida de DB que reconstruye solo una partición densa;
- 2.048 updates en una tabla de 4.096 filas y 32 particiones, comparando filas
  y resumen contra rebuild completo;
- igualdad del `TransactionId`, longitud y wire antes/después de
  encode/decode, más rechazo de todos los prefijos truncados.

Al cerrar este informe pasaron `cargo fmt --check`, la suite completa del
workspace con todos los features y targets, Clippy con warnings denegados y
Rustdoc con warnings denegados. Esto incluye los 11 tests de
`rc_database_reconciliation`, los 10 de `rc_summary_tree`, todos los
benchmarks registrados en modo test y cinco gates ignorados por diseño.

## Campañas ejecutadas

| Campaña | Celdas precisas | Procesos | Observaciones | Finalidad |
|---|---:|---:|---:|---|
| [`bulk-scaling-v1`](../../validation/benchmarks/runs/pre-rc-bulk-scaling-pilot-v1/report.md) | 52/57 | 285 | 2.565 | baseline y detección de cuellos |
| [`bulk-scaling-v2`](../../validation/benchmarks/runs/pre-rc-bulk-scaling-pilot-v2/report.md) | 55/57 | 285 | 2.565 | confirmación tras evitar clones dispersos |
| [`bulk-density-frontier-v1`](../../validation/benchmarks/runs/pre-rc-bulk-density-frontier-pilot-v1/report.md) | 18/18 | 90 | 810 | 75 % y 100 % de densidad |
| [`database-streaming-v1`](../../validation/benchmarks/runs/pre-rc-database-streaming-pilot-v1/report.md) | 6/6 | 30 | 270 | confirmación tras streaming de identidad |

En total son 690 procesos independientes y 6.210 observaciones. Todos los
artefactos incluyen manifest, orden de ejecución, datos raw, entorno,
agregados, intervalos y checksums. Las cuatro campañas son `Informative`, con
`claims_allowed=false`.

## Resultados del summary tree

Archivo de 64 MiB, chunks fijos de 4 KiB y reemplazos de 16 bytes distribuidos
entre hojas distintas:

| Hojas editadas | Densidad | Rebuild | Bulk | Adaptativo | Bulk/rebuild |
|---:|---:|---:|---:|---:|---:|
| 1.024 | 6,25 % | 982,12 ms | 67,92 ms | 68,04 ms | 0,0692 |
| 8.192 | 50 % | 981,97 ms | 509,51 ms | 509,53 ms | 0,5189 |
| 12.288 | 75 % | 1.008,50 ms | 752,21 ms | 752,62 ms | 0,7459 |
| 16.384 | 100 % | 1.008,49 ms | 994,84 ms | 991,74 ms | 0,9866 |

A 6,25 % de densidad el bulk es unas 14,5 veces más rápido que rebuild; a 50
% conserva una ventaja de aproximadamente 1,93 veces y a 75 % de 1,34 veces.
La distribución agrupada no altera la conclusión: a 75 % obtuvo 746,47 ms.
El coste escala con hojas únicas y ancestros únicos, no con una repetición
ciega de `O(k log n)`.

## Resultados de base de datos

Las celdas usan 16 particiones, rows sintéticas en memoria y updates con clave
estable. `sc` indica distribución dispersa y `cl`, agrupada.

| Filas / updates | Distribución | Rebuild | Bulk | Adaptativo | Lectura |
|---:|---|---:|---:|---:|---|
| 4.096 / 256 | dispersa | 9,39 ms | 1,74 ms | 1,74 ms | 256 no es un límite; bulk ≈5,4× mejor |
| 65.536 / 4.096 | dispersa | 163,86 ms | 20,71 ms | 20,45 ms | bulk/adaptativo ≈7,9–8,0× mejor |
| 65.536 / 32.768 | dispersa | 166,55 ms | 178,25 ms | 177,86 ms | medición v2 anterior al streaming; cerca del cruce |
| 65.536 / 49.152 | dispersa | 168,13 ms | 267,07 ms | 260,54 ms | el rebuild ya domina |
| 65.536 / 65.536 | global | 167,85 ms | 343,10 ms | 258,46 ms | adaptativo mejora bulk, pero no supera rebuild |

La mejora de streaming se confirmó en los dos extremos. Para 4.096 updates,
los bytes asignados medianos bajaron de 9,52 MB a 4,54 MB y el pico de 2,95
MB a 1,24 MB. Para 65.536 updates, la ruta adaptativa bajó de 181,91 MB a
102,22 MB asignados y de 47,19 MB a 30,65 MB de pico. Son comparaciones
entre campañas informativas, no ratios pareados publicables.

El baseline `database.selected-rebuild-total` reconstruye la tabla objetivo,
pero no valida un envelope transaccional completo. Es por ello un límite
inferior conservador, no una operación semánticamente idéntica: las rutas de
transacción validan namespace/schema/revisión, before images y límites,
mantienen replay idempotente e identidad, y publican atómicamente la revisión.

## Consecuencias para lotes grandes

- En el árbol, un lote grande sigue siendo útil hasta densidades muy altas. No
  se recomienda fragmentar artificialmente un batch solo por superar 256
  ediciones.
- En DB, debe compararse el número de mutaciones con la población de las
  particiones afectadas. Miles de updates dispersos son una carga favorable;
  cambiar la mitad o más de toda una tabla se aproxima a un bulk load y puede
  justificar rebuild.
- La ruta híbrida evita imponer una sola decisión a toda la transacción: puede
  aplicar deltas en particiones dispersas y reconstruir las densas.
- Los umbrales no deben congelarse como constantes universales. Schema, tamaño
  de fila, número de particiones, encoder, hardware y distribución cambian el
  punto de equilibrio.

## Límites de la evidencia

- Host interactivo: resultados aptos para decisiones internas, no para claims
  de publicación.
- DB sintética en memoria: no incluye planner SQL, índices secundarios, WAL,
  `fsync`, red, bloqueo, concurrencia ni jerarquía de caché de un motor real.
- Un único campo/encoder en estas celdas (`GF(2^128)` y encoder binario), chunk
  de 4 KiB, 16 particiones y updates de 16 bytes en el árbol.
- El frontier exacto de DB después del cambio de streaming no se ha localizado
  todavía; solo se revalidaron 6,25 % y 100 %.
- Las igualdades de firmas continúan significando `Indistinguishable`; las
  filas/chunks exactos y sus before images son la autoridad.

## Siguiente cierre de estas dos secciones

1. Repetir `bulk-scaling-v2`, `bulk-density-frontier-v1` y
   `database-streaming-v1` como `Controlled` en x86-64 dedicado y después en
   AArch64 sin variar workloads.
2. Añadir puntos DB intermedios posteriores al streaming (12,5 %, 25 %, 37,5
   % y 50 %) para calibrar una política por schema/particionado, sin convertir
   esa calibración en default universal.
3. Validar escalas de al menos 1 GiB de árbol y 1 millón de filas, conservando
   memoria pico y coste de creación de transacción como métricas de primer
   nivel.
4. En una fase posterior y separada, llevar el adapter DB a WAL/restart y
   concurrencia real; no atribuir esas propiedades al benchmark en memoria.
