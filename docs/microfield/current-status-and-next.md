# Estado actual y siguiente plan

Fecha de revisión integral: 10 de agosto de 2026.

Este documento es la fotografía autoritativa del proyecto. Los planes e
informes de fases conservan decisiones y evidencia histórica, pero no deben
usarse para deducir el estado actual cuando contradigan esta página o el
inventario ejecutable
[`validation/rc/supported-surface-v1.json`](../../validation/rc/supported-surface-v1.json).

## Veredicto ejecutivo

El commit `fe528c4` obtuvo un dictamen técnico de **consumo interno
condicionado**. B.1–B.3 ya están implementadas y ejecutadas: el harness cubre
todas las familias en smoke y la matriz profunda obtuvo 44/44 celdas precisas,
22 comparaciones pareadas y ocho curvas. Sobre esa base, los checkpoints hasta
`b9d7ef1` ampliaron específicamente el escalado bulk del summary tree y de la
DB y conservaron cuatro campañas adicionales: 690 procesos, 6.210
observaciones y 131/138 celdas precisas. La promoción e integración de la RC
siguen aplazadas hasta replicar la evidencia en un host dedicado `Controlled`;
las ejecuciones actuales son `Informative` y bloquean por diseño los claims
externos.

La implementación funcional hasta RC.6 está integrada en `main`: campos
finitos, generación estática y contextos runtime, firmas homomórficas,
snapshots, deltas, árbol de resúmenes, base de datos, reconciliación acotada,
pipeline de grafos, canonización exacta presupuestada y DAG canónico. La
corrección local y remota es fuerte y reproducible.

La marca y el crate raíz se llaman ahora **Algesum**. Los identificadores wire
`MFRW`, `MFTX`, `MFTL` y sus separadores históricos se conservan por
compatibilidad de datos; no son nombres de producto ni garantías
criptográficas.

RC.7–RC.10 están implementados y verdes local y remotamente: inventario de
corrección, property tests, fuzzing continuo, 37 SLO, regresión máxima del 3
%, break-even, fallback ejecutable, consumidor externo, restart/migración,
runbook y dictamen reproducible. El run
[`31331474150`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/31331474150)
validó el mismo commit limpio en x86-64/AArch64 y RC.10 emitió
`ReadyForInternalUse`.

### Línea de producto que queda por cerrar

El nombre histórico «hash homomórfico» designa aquí una **firma o resumen
algebraico no criptográfico**. La API mantenida usa nombres que muestran su ley
—aditiva, secuencia o multiconjunto— para no sugerir autenticación, resistencia
adversarial ni pruebas de pertenencia que el sistema no ofrece.

El vertical de base de datos no está pendiente de implementación inicial.
RC.5 ya entregó schema y filas canónicas, `PartitionedDatabase`, transacciones
versionadas `MFTX`, log/replay `MFTL`, firmas por partición y reconciliación
acotada `MFRS`. La ampliación bulk ya elimina el falso límite absoluto de 256:
coalesce hojas y ancestros en el árbol, agrupa deltas por partición, evita
clones completos en particiones dispersas, ofrece selección híbrida por
densidad y deriva en streaming la identidad transaccional. En el piloto
informativo, 4.096 updates sobre 65.536 filas son unas ocho veces más rápidos
que el rebuild de referencia.

La primera integración PostgreSQL real ya está ejecutada: 18/18 transacciones
verificadas sobre 65.536 filas y lotes hasta 65.536, con igualdad exacta de
filas y resúmenes frente a rebuild. También existe un adaptador que separa
posiciones externas dispersas de revisiones contiguas, conserva checkpoint y
falla de forma cerrada ante redelivery conflictiva. La evidencia está en
[`algesum-postgresql-results.md`](algesum-postgresql-results.md). Lo que queda
para madurar estas rutas es:

La campaña integral C1 posterior amplió el smoke a 65 celdas: seis leyes de
firma con build/composición, payloads, campos, deltas, árboles, DB,
reconciliación, grafos fast/exact/DAG y tooling. Las 65 terminaron precisas bajo
el umbral relajado de smoke. PostgreSQL también llegó a un millón de filas en
clustered, strided y hotspot con igualdad exacta en todos los commits. Tras
eliminar una copia autoritativa innecesaria del laboratorio, 256 cambios sobre
un millón costaron 8,1–11,9 ms y el proceso denso alcanzó aproximadamente 1,44
GiB de RSS. Véanse
[`pre-rc-comprehensive-smoke-results.md`](pre-rc-comprehensive-smoke-results.md)
y el
[`plan integral`](pre-rc-comprehensive-campaign-plan.md).

C2 ya está ejecutado: 47 celdas de firmas/grafos, 470 procesos y 7.050
observaciones, con 44/47 celdas precisas. Confirma composición K=1..4,
fragmentación K=4 hasta 1.024 operandos alternantes, estabilidad por vértice en
tres familias de grafos y una ventaja incremental de 2,79–2,96x para editar
una etiqueta. PostgreSQL añadió 33/33 muestras exactas a un millón de filas.
El límite detectado está en alta densidad. El A/B posterior mostró que el
rebuild completo tampoco gana a la ruta por particiones ya optimizada, por lo
que queda opt-in; esta decisión y la telemetría exacta de grafos ya están
cerradas. Véase
[`pre-rc-comprehensive-pilot-results.md`](pre-rc-comprehensive-pilot-results.md).

- replicar en entorno controlado tanto la campaña general como las nuevas
  matrices de densidad bulk;
- corregir y repetir la selección DB densa; ejecutar 10 millones sólo en un
  host dedicado con memoria suficiente;
- reevaluar después la integración y fijar un checkpoint recuperable;
- extender el consumidor PostgreSQL desde commits controlados a logical
  decoding/WAL y probar 16–256 clientes concurrentes, crashes y lag;
- importar NYC TLC para canonicalización y carga masiva de datos heterogéneos;
- implementar y ejecutar la C3 extensiva sobre campos, engines, firmas,
  estructuras persistentes, reconciliación, DB y grafos, con 12.000+ casos
  semánticos, 2.500–4.000 celdas de timing y 150–300 escenarios;
- decisión de congelar la reconciliación v1 como conjuntos o diseñar una v2
  para multiplicidad;
- completar licencia, seguridad, semver, advisories/SBOM y packaging externo.

Por tanto, el siguiente trabajo pre-RC combina la réplica controlada de la
evidencia ya reproducible con WAL/concurrencia y corpus real sobre la
integración PostgreSQL existente. Grafos permanece como otro consumidor
importante, no como sustituto de esa línea de producto.

## Base auditada

| Elemento | Estado comprobado |
|---|---|
| Candidato funcional RC.10 | `fe528c4f663aad9a2a07ab8c1f44b6e6e13a916a` |
| Integración en `main` | `d0f4fcdb0cc0c0e4e18b12b0b33ed37389c43b47`, PR [#1](https://github.com/alfredoVallejoM/homomorphic-hash-rs/pull/1) |
| Checkpoint | tag anotado `internal-rc6-integrated` sobre el merge |
| Rama de trabajo RC | `rc/rc7-correctness`, creada desde la línea base integrada |
| Validación RC.10 | run `31331474150`, x86-64/AArch64, `ReadyForInternalUse` |
| Harness publicable B.2 | `42874c7`, 21 celdas smoke y regeneración desde raw |
| Piloto B.3 | evidencia en `49828e3`, 6.600 observaciones, 42/44 celdas precisas |
| Campaña profunda B.3 | evidencia en `36ec77d`, 66.500 observaciones, 44/44 precisas, `Informative` |
| Bulk tree/DB | código hasta `73a14cc`; cuatro campañas hasta `b9d7ef1`, 6.210 observaciones, 131/138 precisas |
| Workspace | cinco paquetes Cargo; dos productos, dos laboratorios privados y un fixture generado |
| Tamaño actual | 624 ficheros versionables y aproximadamente 122 MiB; 104 MiB son `data/` |
| Implementación Rust | aproximadamente 93.000 líneas |
| Documentación Markdown | aproximadamente 19.800 líneas |

Para iniciar trabajo nuevo debe usarse una rama creada desde `main`, no
continuar sobre `agent/h2-5-verified-profiles-pmull`, aunque hoy ambos árboles
sean idénticos.

## Estado por capacidad

| Área | Estado | Expectativa válida |
|---|---|---|
| `microfield` estático | Soportado | campos binarios y primos mantenidos, encoding canónico y fallback portable |
| Generadores binario/primo | Soportado | tipos nominales certificados; `ProbablePrime` no autoriza generación estática |
| Campos runtime | Condicionado | contexto, assurance y límites deben viajar con cada operación/persistencia |
| Firmas aditiva, secuencia y multiset | Soportado | resúmenes algebraicos no criptográficos; igualdad significa `Indistinguishable` |
| Firmas bidireccionales y multievaluadas | Experimentales | útiles como canales adicionales, sin promesa de ausencia de colisiones |
| Tracking exacto | Soportado | conserva la fuente y paga memoria O(n) |
| Residuales | Restringido | ecuación algebraica; nunca prueba de pertenencia ni autorización de borrado |
| Deltas y journals | Soportado | revisión, preflight y commit atómico en memoria; no prometen durabilidad frente a caída |
| Archivo y summary tree | Soportado | bulk atómico coalesce hojas/ancestros; cambio de fronteras activa rebuild explícito |
| DB y reconciliación | Soportado con límites | deltas agrupados e híbridos por partición; filas/versiones exactas; adaptador change-stream condicionado; reconciliación v1 rechaza multiplicidad |
| Filtros de grafos | Soportado como evidencia negativa | pueden rechazar o devolver `Indistinguishable`; nunca crean identidad |
| `Microcanon` | Condicionado | exacto solo al completar presupuesto; de otro modo `Inconclusive` |
| DAG canónico y adapters | Condicionado | reutilización únicamente tras igualdad de bytes canónicos exactos |
| Legado, dominios y harness | Compatibilidad/experimental | no son la ruta recomendada para consumidores nuevos |
| Publicación externa | No preparada | falta licencia, metadata, packaging, semver y política de soporte |

La lista machine-readable completa está en
[`supported-surface-v1.json`](../../validation/rc/supported-surface-v1.json).

## Evidencia reproducida en esta auditoría

Durante esta auditoría se ejecutó:

```text
cargo fmt --all -- --check
cargo test --workspace --all-features --all-targets --locked
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --locked --no-deps
python3 tools/fetch_graph_corpus.py --offline
cargo test -p algesum --all-features --locked \
  --test external_graph_corpus -- --ignored --nocapture
cargo test -p algesum --all-features --release --locked \
  --test graph_canonical \
  microcanon_matches_every_simple_graph_isomorphism_class_at_six_vertices \
  -- --ignored --exact
cargo run --release -p microfield-validation-lab --locked -- semantic ...
```

Resultados:

- formato, Clippy y Rustdoc sin warnings;
- 857 funciones de test descubiertas y ocho doctests;
- todos los tests ordinarios pasaron;
- los cinco tests `ignored` también pasaron al ejecutarlos con sus
  precondiciones: cuatro corpus externos y el exhaustivo de 32.768 grafos;
- el exhaustivo reprodujo las 156 clases simples de orden seis;
- JSON y CSV semánticos regenerados fueron idénticos byte a byte a los
  artefactos versionados;
- todos los benchmarks registrados compilaron y completaron su ejecución en
  modo test; estas ejecuciones no sustituyen una campaña estadística release.

La ampliación posterior de lotes añadió pruebas diferenciales de 200 commits
bulk del árbol, rechazo atómico de solapamientos, política DB híbrida y una
transacción de 2.048 updates sobre 4.096 filas. Las campañas multidimensionales
de tamaño, densidad y distribución están interpretadas en
[`pre-rc-bulk-scaling-results.md`](pre-rc-bulk-scaling-results.md).

La integración remota posterior al merge, run
[`30910486012`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30910486012),
terminó con sus 13 jobs verdes. Incluyó MSRV 1.89, matriz de features y
`no_std`, Miri, ASan en x86-64, hardware PMULL real en AArch64, artefactos
deterministas, RC de firmas y grafos y F6.V reproducible en x86-64 y AArch64.

## Fortalezas actuales

- Separación semántica entre campos, firmas algebraicas, filtros y autoridad
  exacta.
- Identidades y wires versionados con parsers acotados y fail-closed.
- Fallback portable y selección ISA conservadora; ningún backend se promueve
  solo por estar disponible.
- Cinco fronteras `unsafe` confinadas, autenticadas y cubiertas por
  diferenciales, Miri/ASan y auditoría de ensamblado.
- Tests transaccionales sistemáticos: los errores representables conservan
  salida, estado y revisión.
- Validación matemática y estructural independiente mediante Sage/nauty,
  modelos lentos, corpus adversariales y regeneración determinista.
- Features que permiten consumir firmas sin grafos y runtime sin legado.

## Riesgos y deuda abierta

### Bloquean la integración del checkpoint

1. Falta implementar y ejecutar la campaña C3 extensiva como `Controlled` en
   hosts Intel x86-64, AMD x86-64 y AArch64; la evidencia `Informative`,
   incluidas las matrices bulk nuevas, no autoriza claims públicos.
2. La rama validada aún no se ha integrado en `main`.
3. Falta el run post-merge y un tag anotado sobre el merge verde.

### Bloquean publicación externa, no experimentación interna

1. No hay `LICENSE`, `SECURITY.md`, `CONTRIBUTING.md` ni `CHANGELOG.md`.
2. El paquete raíz ya declara repository, homepage, documentation,
   `rust-version` y un boundary `include`, pero no puede declarar una licencia
   inexistente; `microfield` continúa con `publish = false`.
3. El boundary raíz ya excluye `data/`, fuzz y fixtures del archive. El dry-run
   completo queda bloqueado externamente hasta disponer de una fuente de
   registro o git para `microfield`.
4. No hay gate de advisories/SBOM ni política explícita de actualizaciones de
   dependencias.
5. No se ha definido semver, compatibilidad de wires entre releases ni ventana
   de soporte pública.

### Deuda de producto y repositorio

- `legacy` ya no forma parte de los defaults; su compatibilidad sigue
  disponible solo mediante feature explícita.
- `workspace.default-members` contiene solo `microfield`: un `cargo test` sin
  `--workspace` no valida el producto raíz, el laboratorio ni el fixture.
- Cuatro benchmarks históricos no registrados contienen `TODO`; están fuera
  de los targets mantenidos, pero deben archivarse o eliminarse de una futura
  distribución.
- La política de publicación transaccional de artefactos no equivale a
  durabilidad frente a caída, coordinación multiproceso ni atomicidad entre
  filesystems.
- La canonización exacta conserva complejidad exponencial en el peor caso y
  cualquier agotamiento debe propagarse como `Inconclusive`.
- Los baselines químicos, de redes e hipergrafos caracterizan adapters; no
  demuestran equivalencia científica general.
- La copia local ocupa decenas de GiB por artefactos `target/` anidados. Están
  ignorados y no afectan al repositorio, pero conviene tratarlos como caché
  regenerable.

## Siguiente orden de trabajo

### 1. Investigar y congelar el protocolo pre-RC — cerrado

- separar el gate RC.8 de la evidencia destinada a publicación;
- definir unidad experimental, repetición adaptativa, estadística, entorno,
  baselines, matriz completa y límites de claims;
- mantener explícita la clasificación no criptográfica.

Salida obtenida: protocolo versionado y auditable en B.1.

### 2. Implementar el harness publicable — cerrado

- conservar RC.8 como gate rápido de regresión interna;
- añadir workers independientes, raw JSONL, calibración, intervalos
  bootstrap, pairing y metadatos de hardware;
- ejecutar smoke de todas las familias en CI.

Salida requerida: un commit limpio regenera agregados desde observaciones
crudas.

Estado: implementado y publicado en B.2. El smoke cubre 21 celdas en 42
procesos aislados, conserva 210 observaciones y regenera los mismos checksums.
CI repite el gate en x86-64/AArch64; sus cifras son `Smoke`, no publicables.

### 3. Ejecutar piloto, escalabilidad y comparaciones — cerrado informativo

- medir al menos cinco escalas en directo/incremental/rebuild;
- ejecutar el piloto completo y una campaña controlada o informativa;
- publicar incertidumbre, puntos de equilibrio y resultados inconclusos.

Salida obtenida: 6.600 observaciones de piloto y 66.500 profundas; 44/44 celdas
profundas precisas, 22 ratios pareados y ocho curvas regenerables. Aquella
campaña caracterizó la API de edición individual: sus cruces no deben
extrapolarse a un lote coalescido. La extensión bulk añadió 6.210 observaciones
en 138 celdas: el árbol de 64 MiB conserva ventaja hasta 75 % de hojas tocadas
y llega a paridad al 100 %; la DB de 65.536 filas mejora unas ocho veces con
4.096 updates, pero a densidad total el rebuild domina. Véanse
[`pre-rc-b3-benchmark-results.md`](pre-rc-b3-benchmark-results.md) y
[`pre-rc-bulk-scaling-results.md`](pre-rc-bulk-scaling-results.md).

### 4. Implementar y ejecutar C3 extensiva — en curso

- P0 completado: ledger activado, inventario explícito de operaciones y huecos,
  generador determinista de cruces primarios/covering arrays y schemas
  versionados;
- F1/F2 base completadas en preflight: 30 workloads de campos, 30 celdas, 60
  procesos y 300 observaciones; su cierre posterior añade referencias
  independientes, portable/forzado, backends x86 explícitos, seis patrones y
  reducción contra `BigUint`;
- F3–S3 completadas en preflight: 26 operaciones nuevas, 2.112 celdas
  publicables generadas, 56 celdas Smoke, 112 procesos y 560 observaciones;
  56/56 precisas bajo el umbral exploratorio y cero checksums inestables;
- el inventario combinado F1–S3 alcanza 2.610 celdas C3-Scaling y satisface el
  suelo normativo;
- T1/R1/D1 completadas en preflight: 374 celdas nuevas, 16 variantes, cero
  checksums inestables y 16/16 precisas tras recalibrar restore;
- G1/G2/X1/X2 completadas: matrices sintéticas y exactas de grafos, DAG,
  cinco corpora fijados, once familias wire y consumidor externo; el inventario
  acumulado alcanza 3.243 celdas;
- D2 ejecutada como preflight externo: 1.888 commits WAL observados, 1–32
  clientes, drenaje 2×/5×, migración y recuperación exacta tras restart;
- cierre F1/F2 ejecutado: 366 celdas nuevas definidas, 85 preflights más cinco
  calibraciones, 206 procesos y 1.330 observaciones; todas las variantes tienen
  una ejecución precisa y el inventario acumulado alcanza 3.609 celdas;
- consolidación ejecutada: 229 celdas/calibraciones locales, 485 procesos y
  2.770 observaciones en total, sin divergencias semánticas ni checksums
  inestables; sólo `postgres.backpressure-soak` permanece ausente del
  inventario de operaciones;
- C3-C0 ejecutado: 221.342 casos/controles deterministas, 22 tests de
  propiedades/modelos y 15.000 ejecuciones fuzz sin crash, timeout ni hallazgo
  de AddressSanitizer;
- completar C3-Scaling y C3-Systems;
- replicar los claims en Intel x86-64, AMD x86-64 y AArch64 dedicados;
- decidir go/no-go con cobertura, efectos, intervalos, límites e inconclusos;
- solo entonces abrir PR, integrar, ejecutar CI post-merge y etiquetar.

Especificación: [`c3-extensive-campaign-plan.md`](c3-extensive-campaign-plan.md).
Evidencia P0:
[`c3-p0-implementation-and-preflight-report.md`](c3-p0-implementation-and-preflight-report.md).
Evidencia F3–S3:
[`c3-f3-s3-implementation-and-preflight-report.md`](c3-f3-s3-implementation-and-preflight-report.md).
Evidencia T1/R1/D1:
[`c3-t1-r1-d1-implementation-and-preflight-report.md`](c3-t1-r1-d1-implementation-and-preflight-report.md).
Evidencia G1/G2:
[`c3-g1-g2-implementation-and-preflight-report.md`](c3-g1-g2-implementation-and-preflight-report.md).
Evidencia X1/X2:
[`c3-x1-x2-implementation-and-preflight-report.md`](c3-x1-x2-implementation-and-preflight-report.md).
Evidencia D2:
[`c3-d2-postgresql-systems-preflight-report.md`](c3-d2-postgresql-systems-preflight-report.md).
Evidencia de cierre F1/F2:
[`c3-f1-f2-closure-preflight-report.md`](c3-f1-f2-closure-preflight-report.md).
Consolidación y secuencia restante:
[`c3-preflight-consolidated-results-and-controlled-next.md`](c3-preflight-consolidated-results-and-controlled-next.md).
Evidencia C3-C0:
[`c3-c0-semantic-results.md`](c3-c0-semantic-results.md).

### 5. Base de datos real

- el adapter LSN/revisión, WAL lógico reproducible, concurrencia hasta 32,
  crash/restart, migración y rebuild autoritativo ya tienen preflight;
- implementar un consumidor del protocolo de replicación y separar writers de
  readers;
- medir transacción, WAL, fsync, replay, reconciliación y fallback end-to-end.

Salida requerida: equivalencia exacta tras restart y curvas de capacidad con
I/O real.

### 6. Publicación externa

- elegir licencia y añadir políticas de seguridad/contribución/changelog;
- fijar semver, MSRV y compatibilidad de wires;
- añadir advisories/SBOM y preparar una fuente publicable para `microfield`;
- verificar archives desde un consumidor limpio.

Salida requerida: package audit completo y claims públicos acotados.

### Disciplina

Cada fase terminará con código/documentación, validación local, commit, push y
CI remoto verde. El detalle normativo vive en
[`post-rc-benchmark-and-publication-plan.md`](post-rc-benchmark-and-publication-plan.md).
El protocolo estadístico normativo está en
[`pre-rc-benchmark-protocol.md`](pre-rc-benchmark-protocol.md).

## Autoridad documental

1. Esta página define el estado y el siguiente orden.
2. `validation/rc/supported-surface-v1.json` define la clasificación ejecutable
   de capacidades.
3. [`contracts.md`](contracts.md), [`architecture.md`](architecture.md), los
   ADR y los schemas versionados definen contratos técnicos.
4. [`release-candidate-readiness-plan.md`](release-candidate-readiness-plan.md)
   registra RC.7–RC.10 y su dictamen remoto.
5. [`pre-rc-benchmark-protocol.md`](pre-rc-benchmark-protocol.md) define los
   gates de benchmark que preceden a la promoción RC.
6. [`post-rc-benchmark-and-publication-plan.md`](post-rc-benchmark-and-publication-plan.md)
   conserva el backlog de integración, DB real y publicación.
7. Los informes `*-final-report.md` y planes de fases cerradas son evidencia
   histórica; sus cifras y frases de “siguiente paso” conservan el contexto de
   su fecha.

El índice completo y la política de lectura están en
[`docs/microfield/README.md`](README.md).
