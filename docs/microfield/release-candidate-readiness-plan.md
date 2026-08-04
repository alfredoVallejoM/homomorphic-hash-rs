# Plan maestro para alcanzar la release candidate técnica

Fecha: 4 de agosto de 2026.

Estado: RC.0–RC.6 completados localmente; RC.7 es el siguiente workstream.

Antes de cerrar RC.7 se ejecutará el gate transversal de integración remota
descrito en
[`github-integration-and-remote-validation-plan.md`](github-integration-and-remote-validation-plan.md).
Ese gate preserva el historial, integra la rama candidata en `main`, completa
la cobertura CI y habilita las campañas remotas pesadas sobre una base
recuperable.

La evidencia de los dos primeros workstreams está en
[`rc-0-rc-1-implementation-report.md`](rc-0-rc-1-implementation-report.md).
El cierre de la fachada de firmas está en
[`rc-2-signature-api-report.md`](rc-2-signature-api-report.md).

## 1. Objetivo y alcance

Este documento enumera todo lo que falta para convertir el estado actual del
workspace en una **release candidate técnica para consumo interno**. La RC debe
permitir depender de la biblioteca desde otro proyecto sin APIs privadas,
reconstrucciones manuales ni conocimiento de sus representaciones internas.

La RC cubre cuatro pilares de producto:

1. infraestructura de campos finitos;
2. firmas homomórficas no criptográficas como API de primer nivel;
3. protocolos de delta para archivos, bases de datos y árboles jerárquicos;
4. firmas estructurales, comparación y canonización de grafos.

La publicación externa —licencia definitiva, crates.io, semver 1.0, soporte
comercial y SLA público— seguirá siendo una fase posterior. Esta separación no
rebaja los gates de corrección, reproducibilidad o estabilidad interna.

## 2. Reglas no negociables

- Las firmas no son criptográficas: no autentican, no prueban pertenencia y no
  ofrecen resistencia adversarial.
- Una igualdad compacta significa `Indistinguishable`, salvo que una garantía
  acotada aplicable o una comparación exacta indique expresamente otra cosa.
- Los campos finitos son infraestructura pública reutilizable, no un detalle
  interno del motor de grafos.
- Los grafos son un vertical consumidor; no determinan toda la API.
- Ningún residual algebraico se presentará como prueba.
- Ningún delta compacto autoriza por sí solo la retirada de un dato.
- Toda identidad liga campo, encoding, ley, parámetros y schema.
- Todo error representable deja estado, revisión y salida intactos.
- La selección ISA permanece fuera del elemento y se hace una vez por engine o
  lote.
- Ninguna optimización entra en `Auto` sin corrección diferencial y medición
  reproducible en hardware compatible.
- No se aceptará una RC narrativa: el resultado será un artefacto versionado y
  reproducible.

## 3. Inventario consolidado actual

### 3.1 Campos finitos

Disponible:

- `F2` y campos binarios mantenidos de 128 y 256 bits;
- campos primos mantenidos F251, Goldilocks y primo genérico de 256 bits;
- `BinaryFieldFactory` y `PrimeFieldFactory` para tipos externos generados;
- `DynField`/`DynElement` para campos binarios y primos runtime validados;
- identidades `FieldId` y `ArtifactId`;
- encoding canónico y traits segregados;
- certificados de Rabin/Pocklington y assurance de validación;
- engines portable, batch, packed y backends ISA auditados;
- bridges static/runtime y perfiles ISA externos verificados;
- referencia Sage y vectores externos.

Estado: funcional y avanzado. Falta convertir la selección campo/perfil para
firmas en una política de producto y cerrar una matriz RC única sobre todas las
familias admitidas.

### 3.2 Firmas homomórficas

Disponible:

- `AdditiveSignature`;
- `SequenceSignature`;
- `BidirectionalSequenceSignature`;
- `MultisetSignature`;
- `MultiEvaluationMultisetSignature`;
- `MultiEvaluationSequenceSignature`;
- `TrackedSequence` y `TrackedMultiset`;
- `AlgebraicResidual`;
- equivalentes runtime bajo `dynamic-fields`;
- encoders canónico, binario, primo, multicanal y adapters legacy;
- `SignatureLaw`, `SignatureAssurance`, `EncoderId`, `SignatureId` y
  `SignatureContext`;
- wire compacto `MFSG` v1;
- ingestión por lotes transaccional;
- 145.636 ecuaciones metamórficas/de partición verificadas;
- colisiones y degeneraciones mínimas congeladas.

Estado: primitivas validadas, todavía no producto completo. Falta estabilizar
la fachada, los perfiles, snapshots, protocolos y consumidores reales.

### 3.3 Reconciliación

Disponible:

- decoder acotado basado en evaluaciones del polinomio característico;
- recuperación validada sobre 63.232 pares;
- rechazo comprobado fuera de la cota declarada.

Estado posterior a RC.5: decoder público mantenido con perfil, wire y límites.
V1 soporta conjuntos y rechaza multiplicidades.

### 3.4 Grafos y canonización

Disponible:

- modelo relacional tipado;
- firmas rápidas y perfiles globales/relacionales;
- histograma exacto de grados con canal multiconjunto;
- `AdaptiveGraphPipeline` de seis niveles;
- `Microcanon` exacto y mappings verificados;
- block-cut, bosques exactos, matcher pareado y fallback fail-closed;
- `GraphDelta` transaccional e incrementalidad de labels;
- corpus exhaustivo n=6, nauty n=8, CFI, SRG y otros adversariales;
- DAG y adapters planificados para cierre.

Estado: núcleo exacto y pipeline utilizables bajo condiciones. Falta cerrar la
persistencia canónica, adapters de consumo y gates de capacidad.

## 4. Dos ampliaciones obligatorias de firmas

### 4.1 Firmas como producto público de primer nivel

La RC expondrá las firmas independientemente del módulo de grafos. La fachada
recomendada no ocultará la ley bajo un tipo genérico llamado “hash”. Cada estado
publicará:

```text
SignatureLaw
SignatureAssurance
FieldId / DynFieldId
EncoderId
SignatureId
parámetros algebraicos
cardinalidad o longitud
wire schema
límites aplicables
```

Trabajo pendiente:

- feature pública `signatures` independiente de `graph`;
- feature `dynamic-signatures` dependiente de campos runtime;
- builders coherentes para cada familia;
- profiles mantenidos K=1/K=2/K=4 con coste y assurance documentados;
- selección `SignatureFieldProfile` según ley, característica, cardinalidad,
  representación y backend;
- paridad completa entre API estática y runtime;
- snapshots compactos y snapshots exactos rastreados como schemas distintos;
- APIs batch de ingestión, combinación y restauración;
- guía de migración desde agregadores legacy;
- deprecación explícita de `ProofGenerator`, `ProofVerifier` y aliases que
  sugieran pertenencia o seguridad;
- ejemplos externos de suma, secuencia, multiconjunto, multievaluación y
  tracking.

### 4.2 Deltas para archivos, bases de datos y árboles jerárquicos

La RC añadirá tipos de delta segregados por ley:

```text
AdditiveDelta
MultisetDelta
SequenceAppend
SequenceTrim
SequenceRangeDelta
```

Todos compartirán:

```text
delta_schema
SignatureContext
application_namespace
source_revision
target_revision
operation_count
payload específico de la ley
```

Se distinguirán tres verificaciones:

- `AlgebraicConsistency`: recomposición de la ecuación;
- `SourceAuthorized`: la fuente exacta valida revisión y retiradas;
- `ExactRebuild`: reconstrucción y comparación exactas.

#### Archivos

- `FileChunkProfileId` y chunking versionado;
- framing de contenido y longitud;
- append/truncate/insert/remove/replace de rangos;
- árbol o rope para recomponer O(log n) nodos por edit local;
- snapshot, journal, replay e idempotencia;
- fallback cuando un cambio desplaza las fronteras de chunks;
- comparación periódica contra relectura completa.

#### Bases de datos

- `DatabaseSchemaId`;
- encoding de clave, tipos, columnas, `NULL`, collation y versión;
- `TransactionDelta` con inserciones, borrados y before/after images;
- integración con revisión o LSN;
- particiones combinables y multiplicidad explícita;
- reconstrucción diferencial desde la fuente autoritativa;
- replay, rollback y recuperación tras crash.

#### Árbol jerárquico

- `HomomorphicSummaryTree` con topología compatible con árboles Merkle;
- perfil identificado de fanout, orden, padding, hojas vacías y framing;
- nodos con firma, ley, longitud/cardinalidad y revisión;
- actualización de hoja y recomposición hasta raíz;
- snapshots y journal transaccional;
- integración opcional como canal adicional de un árbol Merkle existente;
- documentación inequívoca: la raíz algebraica no es una raíz autenticada.

El análisis detallado se encuentra en
[`phase-6-signature-delta-audit.md`](phase-6-signature-delta-audit.md).

## 5. Workstreams hasta RC

### RC.0 — congelar inventario y claims — completado localmente

Entregables:

- allowlist `Supported`, `Experimental`, `LegacyAdapter`, `Rejected`;
- mapa de features y dependencias;
- inventario de todos los tipos públicos;
- tabla de garantías, límites y costes;
- retirada de ejemplos o nombres engañosos.

Gate: todo símbolo público tiene owner, estado y contrato; ningún flujo
recomendado depende del legado.

### RC.1 — cerrar infraestructura de campos — completado localmente

Entregables:

- matriz de traits por campo mantenido, generado y runtime;
- perfiles de generación y validación congelados;
- locks y artefactos reproducibles;
- equivalencia static/runtime por `FieldId`;
- política de selección portable/ISA/packed;
- guía para crear y consumir un campo externo;
- límites de grado, modulus, memoria y validación.

Gate:

- regeneración determinista con diff vacío;
- leyes y vectores de referencia en todos los campos admitidos;
- ningún backend modifica encoding, identidad o semántica;
- fallback portable disponible para todo perfil soportado.

### RC.2 — estabilizar API de firmas — completado localmente

Entregables:

- fachada y features independientes de grafos;
- builders y nombres homogéneos;
- profiles de campo/ley/K;
- wire y snapshots versionados;
- paridad static/runtime;
- batch y restauración transaccional.

Gate: suite genérica única sobre cada combinación admitida, compile-fail para
contextos incompatibles y consumidor externo sin APIs privadas.

### RC.3 — implementar núcleo de deltas — completado localmente

Entregables:

- envelopes y errores de revisión;
- delta aditivo y de multiconjunto;
- append/trim ordenado;
- preflight completo y commit atómico;
- journal y replay idempotente;
- clasificación de verificación.

Gate: cada secuencia aleatoria de deltas coincide con rebuild después de cada
paso y todo fallo conserva bytes y revisión anteriores.

Evidencia: `docs/microfield/rc-3-delta-core-report.md`.

### RC.4 — archivos y árbol jerárquico — completado localmente

Entregables:

- adapter de chunks;
- edits de rango mediante árbol/rope;
- `HomomorphicSummaryTree`;
- persistencia y recuperación;
- benchmarks frente a relectura/rebuild.

Gate: cualquier edit admitido produce la misma raíz que reconstruir; un cambio
de hoja toca O(log n) nodos para la forma congelada; los cambios de frontera
activan un fallback correcto.

Evidencia: `docs/microfield/rc-4-summary-tree-report.md`.

### RC.5 — base de datos y reconciliación — completado localmente

Entregables:

- schema de filas y `TransactionDelta`;
- particiones y transacciones versionadas;
- decoder de reconciliación en módulo mantenido;
- límites de universo, diferencia, grado y memoria;
- decisión explícita sobre multiplicidad.

Gate: replay de logs y reconstrucción desde tablas coinciden; reconciliación
recupera exactamente dentro de cota y devuelve error tipado fuera de ella.

Evidencia: `docs/microfield/rc-5-database-reconciliation-report.md`.

### RC.6 — cerrar grafos/DAG/adapters — completado localmente

Entregables:

- schema canónico persistente;
- `lookup → filter → exact → insert/reuse` transaccional;
- DAG exacto para cliques y subredes;
- adapters que no pierdan semántica silenciosamente;
- delta de labels y política medida para cambios topológicos.

Gate: ninguna firma rápida crea una identidad definitiva; toda reutilización
del DAG deriva de bytes canónicos exactos.

Evidencia: `docs/microfield/rc-6-graph-dag-report.md`.

### RC.7 — validación exhaustiva y adversarial

Entregables:

- laws/property tests de campos y firmas;
- fuzzing de manifests, encoders, wires, snapshots, journals y parsers;
- campañas de deltas contra modelo exacto;
- corpus externos reproducibles;
- Miri para portable y sanitizers para ISA/storage;
- fixtures mínimos de toda colisión o divergencia.

Gate: cero divergencias no clasificadas, cero mutaciones parciales y cero
panics ante input externo dentro de los límites publicados.

### RC.8 — rendimiento y capacidad

Escenarios:

- field scalar/batch/packed;
- ingest/merge/concatenate/remove/restore por firma;
- apply/replay/rollback por tipo de delta;
- archivo append y edits aleatorios;
- transacciones de filas y reconciliación;
- actualización de hoja y raíz jerárquica;
- filtros, comparación exacta y DAG de grafos.

Métricas:

- p50/p95/p99, throughput y allocations;
- elementos/s y bytes/s;
- I/O evitado por delta;
- memoria persistente y temporal;
- bytes comunicados;
- coste de generación separado del coste de aplicación;
- punto de equilibrio incremental frente a rebuild;
- overhead de fachada frente a kernel/operación directa.

Gate: SLO por workload y fallback explícito fuera de su región rentable. No se
promociona una optimización con regresión mayor del 3 % en su ruta congelada sin
decisión documentada.

### RC.9 — interoperabilidad y operabilidad

Entregables:

- crate consumidor fixture;
- ejemplos end-to-end de campo externo, firmas, archivo, DB, árbol y grafo;
- runbook de errores, reconstrucción y migración;
- observabilidad de perfil, backend, revisión, fallback e inconclusión;
- compatibilidad x86-64/AArch64;
- inventario de dependencias y package dry-run sin publicar.

Gate: un consumidor limpio compila, persiste, reinicia, aplica deltas y
reconstruye resultados sin tocar módulos privados.

### RC.10 — artefacto go/no-go

Se generará un resultado versionado con:

```text
commit
toolchains
features
hardware/runners
field and signature matrices
corpus manifests
semantic gates
performance gates
known limitations
capability classification
final decision
```

Decisiones posibles:

- `ReadyForInternalUse`;
- `Conditional`, con restricciones ejecutables;
- `NotReady`, con gate bloqueante concreto.

## 6. Matriz de pruebas obligatoria

| Área | Corrección | Metamorfismo | Adversarial | Persistencia | Rendimiento |
|---|---|---|---|---|---|
| campos | leyes y encoding | static/runtime/ISA | módulos límite | locks/vectores | scalar/batch |
| encoders | framing e identidad | partición de input | longitudes/canonicalidad | golden IDs | bytes/s |
| firmas | ley y assurance | merge/asociación/orden | colisiones/ceros | MFSG/snapshot | elem/s |
| deltas | incremental vs rebuild | replay/rollback | revisión/ausencia | journal/crash | apply vs rebuild |
| archivos | contenido y longitud | rechunk/edit sequence | fronteras/ráfagas | restart | I/O evitado |
| DB | estado por transacción | partición/replay | duplicados/schema drift | LSN/snapshot | tx/s |
| árbol | raíz vs rebuild | orden/fanout | forma/collision | nodos/journal | O(log n) |
| reconciliación | recuperación en cota | partición/unión | fuera de cota | envelope | decode/bytes |
| grafos | exacto/oráculo | renumeración | CFI/SRG | DAG | tiers/exacto |

## 7. Gates transversales de calidad

Antes de etiquetar la RC deberán pasar:

```text
cargo fmt --all -- --check
cargo test --workspace --all-features --all-targets
cargo clippy --workspace --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

Además:

- combinaciones mínimas y máximas de features;
- compilación `no_std` de `microfield` donde corresponda;
- Miri en rutas portables seleccionadas;
- ASan/UBSan para backends y storage con `unsafe`;
- CI x86-64 y AArch64 real o runners confiables;
- Sage como oráculo externo en el entorno `laboratorio_np`;
- regeneración determinista;
- `git diff --check` y documentación sin enlaces rotos;
- benchmark guardado fuera del gate semántico y comparado por hardware.

## 8. Riesgos que bloquean RC

1. Usar un residual como autorización de borrado.
2. Comparar estados sin `SignatureContext` completo.
3. Mezclar snapshots rastreados y compactos bajo el mismo schema.
4. Prometer edits arbitrarios de archivo sin índice jerárquico.
5. Aplicar un update de DB sin before image o fuente autoritativa.
6. Llamar Merkle/authenticated a una raíz exclusivamente algebraica.
7. Seleccionar un campo sin considerar característica y ley.
8. Ocultar el coste de generar el delta al medir su aplicación.
9. Persistir una clave de grafo derivada solo de un filtro.
10. Promocionar ISA no ejecutada en hardware compatible.
11. Mantener APIs legacy con nombres probatorios en la ruta recomendada.
12. Aprobar por ausencia empírica de colisiones sin declarar el alcance.

## 9. Orden de ejecución

```text
RC.0 inventario
   │
   ├──► RC.1 campos ────────┐
   │                        │
   └──► RC.2 firmas ────────┤
                            ▼
                    RC.3 núcleo delta
                      │           │
                      ▼           ▼
              RC.4 archivo/árbol  RC.5 DB/reconciliación
                      │           │
                      └─────┬─────┘
                            ▼
                    RC.6 grafos/DAG
                            ▼
                    RC.7 validación
                            ▼
                    RC.8 capacidad
                            ▼
                    RC.9 operabilidad
                            ▼
                    RC.10 go/no-go
```

RC.6 puede avanzar en paralelo con RC.3–RC.5 siempre que no cambie la API base
de firmas. RC.7 empieza desde RC.0, pero solo se cierra cuando todos los
verticales están completos.

## 10. Definition of Done de la RC

La release candidate estará terminada únicamente cuando:

1. campos estáticos, generados y runtime tengan una matriz soportada explícita;
2. las firmas puedan consumirse sin importar módulos de grafos o legacy;
3. toda firma declare ley, assurance, contexto, wire y límites;
4. exista una API de delta transaccional por ley;
5. archivos soporten cambios versionados y recomposición jerárquica;
6. bases de datos soporten before/after images, replay e idempotencia;
7. el árbol jerárquico coincida con rebuild y no reclame autenticidad;
8. reconciliación sea pública, acotada y fail-closed;
9. el DAG de grafos derive identidad solo de canonización exacta;
10. los tests diferenciales, property, fuzz, Miri y sanitizers estén verdes;
11. existan SLO y límites por workload;
12. un consumidor externo complete todos los flujos después de restart;
13. el artefacto RC sea reproducible desde un commit limpio;
14. toda limitación aceptada esté codificada en API, configuración o test.

## 11. Resultado esperado

Al finalizar, la biblioteca ofrecerá una plataforma unificada de campos finitos
certificados y optimizados, firmas homomórficas componibles, actualización por
deltas sobre estructuras persistentes y canonización exacta de grafos. Cada
capa conservará su semántica: los campos aportan álgebra, las firmas aportan
composición compacta, la fuente exacta autoriza cambios y el canonizador aporta
identidad estructural cuando el dominio lo requiere.
