# Estado actual y siguiente plan

Fecha de revisión integral: 9 de agosto de 2026.

Este documento es la fotografía autoritativa del proyecto. Los planes e
informes de fases conservan decisiones y evidencia histórica, pero no deben
usarse para deducir el estado actual cuando contradigan esta página o el
inventario ejecutable
[`validation/rc/supported-surface-v1.json`](../../validation/rc/supported-surface-v1.json).

## Veredicto ejecutivo

El proyecto es un **candidato técnico integrado para consumo interno
condicionado**. No es todavía una release candidate final ni un producto listo
para publicación externa.

La implementación funcional hasta RC.6 está integrada en `main`: campos
finitos, generación estática y contextos runtime, firmas homomórficas,
snapshots, deltas, árbol de resúmenes, base de datos, reconciliación acotada,
pipeline de grafos, canonización exacta presupuestada y DAG canónico. La
corrección local y remota es fuerte y reproducible.

Lo pendiente ya no es completar el núcleo RC.0–RC.6. RC.7–RC.10 están
implementados y verdes localmente: inventario de corrección, property tests,
fuzzing continuo, 37 SLO, regresión máxima del 3 %, break-even, fallback
ejecutable, consumidor externo, restart/migración, runbook y dictamen
reproducible. El resultado local RC.10 es correctamente `Conditional`: falta
publicar el commit limpio y reproducir toda la evidencia en runners remotos
x86-64/AArch64.

### Línea de producto que queda por cerrar

El nombre histórico «hash homomórfico» designa aquí una **firma o resumen
algebraico no criptográfico**. La API mantenida usa nombres que muestran su ley
—aditiva, secuencia o multiconjunto— para no sugerir autenticación, resistencia
adversarial ni pruebas de pertenencia que el sistema no ofrece.

El vertical de base de datos no está pendiente de implementación inicial.
RC.5 ya entregó schema y filas canónicas, `PartitionedDatabase`, transacciones
versionadas `MFTX`, log/replay `MFTL`, firmas por partición y reconciliación
acotada `MFRS`. Lo que quedó a medio cerrar es la maduración conjunta de estas
firmas y protocolos:

- integrar remotamente el fuzzing/property testing ya implementado para
  firmas, snapshots, deltas, transacciones, logs y reconciliación;
- integrar remotamente los SLO ya implementados de ingestión, merge,
  apply/replay, rebuild y reconciliación;
- integrar remotamente el consumidor externo ya implementado con
  almacenamiento, restart, corrupción, migración y reconstrucción;
- aplicar en un motor real el mapeo LSN/revisión fijado por el runbook;
- decisión de congelar la reconciliación v1 como conjuntos o diseñar una v2
  para multiplicidad;
- runbook, observabilidad y artefacto final de go/no-go.

Por tanto, RC.7–RC.10 deben priorizar el cierre del producto de firmas
homomórficas y su vertical de base de datos. Grafos permanece como otro
consumidor importante, no como sustituto de esa línea de producto.

## Base auditada

| Elemento | Estado comprobado |
|---|---|
| Candidato de código | `2f1f1a858adaffd0bde5466dc7e47b3b5064652e` |
| Integración en `main` | `d0f4fcdb0cc0c0e4e18b12b0b33ed37389c43b47`, PR [#1](https://github.com/alfredoVallejoM/homomorphic-hash-rs/pull/1) |
| Checkpoint | tag anotado `internal-rc6-integrated` sobre el merge |
| Rama de trabajo RC | `rc/rc7-correctness`, creada desde la línea base integrada |
| Workspace | cuatro paquetes Cargo; dos productos, un laboratorio privado y un fixture generado |
| Tamaño tras esta revisión | 492 ficheros, aproximadamente 109 MiB; 104 MiB corresponden a `data/` |
| Implementación Rust | 271 ficheros y aproximadamente 84.600 líneas |
| Documentación Markdown | 99 ficheros y aproximadamente 17.800 líneas |

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
| Archivo y summary tree | Soportado | edición local con fronteras fijas; cambio de fronteras activa rebuild explícito |
| DB y reconciliación | Soportado con límites | filas/versiones exactas y reconciliación de conjuntos; v1 rechaza multiplicidad |
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
cargo test -p homomorphic-hash-rs --all-features --locked \
  --test external_graph_corpus -- --ignored --nocapture
cargo test -p homomorphic-hash-rs --all-features --release --locked \
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

### Bloquean la decisión RC interna

1. RC.7–RC.10 aún no tienen evidencia remota del mismo commit limpio en
   x86-64 y AArch64; localmente sus gates están verdes.
2. El artifact RC.10 local es necesariamente `Conditional`. El job remoto ya
   está definido y debe emitir `ReadyForInternalUse` para cerrar la RC.

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

### 0. Fijar esta nueva línea base

- continuar desde `main` y conservar el tag `internal-rc6-integrated` como
  checkpoint histórico;
- revisar y versionar esta actualización documental;
- mantener verde el job agregado `Required gates`.

### 1. RC.7 — robustez adversarial — implementación local completa

- crear targets de fuzz para manifests, encoding, todos los wires y parsers;
- añadir campañas property/diferenciales de deltas, journals, DB, árbol y DAG;
- persistir cada fallo mínimo como fixture;
- definir ventanas nightly y límites de memoria/tiempo reproducibles.

Salida local obtenida: cero divergencias no clasificadas, panics de input o
mutaciones parciales en la ventana ejecutada. Pendiente: gate remoto.

### 2. RC.8 — capacidad y SLO — implementación local completa

- congelar workloads internos representativos;
- medir aplicación y generación de deltas por separado;
- publicar p50/p95/p99, throughput, memoria, allocations y bytes persistidos;
- fijar el punto de fallback incremental/rebuild y ceilings exactos.

Salida local obtenida: 37/37 SLO, dos curvas de break-even y fallback
ejecutable. Pendiente: artifacts remotos x86-64/AArch64 y baseline de PR.

### 3. RC.9 — interoperabilidad y operabilidad — implementación local completa

- construir un fixture consumidor end-to-end para firmas, archivo, DB,
  reconciliación y DAG;
- probar persistencia, restart, corrupción, schema drift y reconstrucción;
- escribir runbook, observabilidad mínima e inventario de dependencias;
- decidir defaults, alcance del legado y estrategia de datasets/packaging.

Salida local obtenida: consumo limpio, persistente y migrable sin módulos
privados ni conocimiento de representaciones internas. Pendiente: matriz
remota x86-64/AArch64.

### 4. RC.10 — decisión reproducible — implementación local completa

- artifact versionado con commit, toolchains, hardware, matrices, corpus,
  gates, SLO y limitaciones;
- precedencia fail-closed con salidas exactas `ReadyForInternalUse`,
  `Conditional` o `NotReady`;
- job requerido que ensambla RC.7–RC.9 y exige evidencia de ambas
  arquitecturas desde un checkout limpio.

Salida local obtenida: `Conditional`, por ausencia deliberada de AArch64,
commit limpio y estado agregado de CI. Pendiente: publicar la rama y obtener
el artifact remoto `ReadyForInternalUse`. Solo después se abrirá una fase
separada de licencia, semver y publicación.

## Autoridad documental

1. Esta página define el estado y el siguiente orden.
2. `validation/rc/supported-surface-v1.json` define la clasificación ejecutable
   de capacidades.
3. [`contracts.md`](contracts.md), [`architecture.md`](architecture.md), los
   ADR y los schemas versionados definen contratos técnicos.
4. [`release-candidate-readiness-plan.md`](release-candidate-readiness-plan.md)
   define RC.7–RC.10 implementados y pendientes de integración remota conjunta.
5. Los informes `*-final-report.md` y planes de fases cerradas son evidencia
   histórica; sus cifras y frases de “siguiente paso” conservan el contexto de
   su fecha.

El índice completo y la política de lectura están en
[`docs/microfield/README.md`](README.md).
