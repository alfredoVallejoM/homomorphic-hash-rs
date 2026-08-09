# Plan ejecutado de integración GitHub y validación remota integral

Fecha de auditoría inicial: 4 de agosto de 2026. Revisión: 9 de agosto de 2026.

Estado: integración ejecutada. La PR
[#1](https://github.com/alfredoVallejoM/homomorphic-hash-rs/pull/1) integró el
candidato `2f1f1a8` en `main` mediante el merge `d0f4fcd`; el tag anotado
`internal-rc6-integrated` fija el checkpoint y el run post-merge
[`30910486012`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30910486012)
terminó verde. Este archivo conserva la auditoría y secuencia originales; no
es el backlog vigente ni autoriza nuevas mutaciones remotas.

## 1. Objetivo

Cerrar la distancia entre el estado desarrollado y una rama `main` realmente
integrable, protegida y verificable. El resultado debe permitir ejecutar en
GitHub tanto gates rápidos de cada cambio como campañas exhaustivas,
adversariales y de rendimiento, sin depender de archivos o credenciales de una
máquina de desarrollo.

La integración debe demostrar simultáneamente:

1. conservación de todo el historial y contenido desarrollado;
2. compilación desde un clon limpio y un lockfile congelado;
3. cobertura remota de campos, firmas, protocolos, grafos y legado mantenido;
4. reproducibilidad de oráculos y artefactos;
5. separación entre corrección bloqueante y rendimiento dependiente de
   hardware;
6. recuperación inequívoca ante una integración defectuosa.

## 2. Fotografía auditada de GitHub

Repositorio: `alfredoVallejoM/homomorphic-hash-rs`.

| Elemento | Evidencia observada | Diagnóstico |
|---|---|---|
| Rama por defecto | `main` en `d0f4fcdb0cc0c0e4e18b12b0b33ed37389c43b47` | contiene el candidato completo |
| Rama candidata | `agent/h2-5-verified-profiles-pmull` en `2f1f1a858adaffd0bde5466dc7e47b3b5064652e` | integrada; no usar como base de trabajo nuevo |
| Relación entre ramas | la candidata es el segundo padre del merge y ambos trees son idénticos | historia preservada sin squash ni rebase |
| Ramas locales/remotas históricas | ancestros del candidato auditado | no se observó trabajo lateral fuera de la integración |
| Commits no alcanzables | ninguno; `git fsck` solo encontró blobs/trees temporales | no hay commits perdidos en el object store local |
| PR | #1 cerrada e integrada el 4 de agosto | revisión formal completada |
| Tags | `internal-rc6-integrated` sobre `d0f4fcd` | checkpoint recuperable disponible |
| Protección/rulesets de `main` | no reauditorados en esta revisión local | mantener como decisión de gobernanza pendiente de verificación |
| Workflows en default | `Microfield`, `F6 external corpus` y `Microfield calibration` | gates PR/push, campaña semanal y calibración manual separados |
| Run del candidato | `30909370895`, 13 jobs verdes | `Required gates` permitió integrar |
| Run post-merge | `30910486012`, 13 jobs verdes | baseline remoto autoritativo de RC.6 integrada |
| Clon limpio remoto | HEAD y tree correctos; `cargo metadata --locked --offline` correcto | lockfile y workspace autocontenidos |
| Compilación limpia | `cargo test --workspace --all-targets --all-features --locked --no-run` correcto | todos los targets publicados son compilables |

La rama candidata contiene también los commits de las ramas H2.1–H2.4, H3,
H4 y todos los commits posteriores de campos, ISA, firmas, validación y grafos.
No hace falta fusionar esas ramas individualmente.

## 3. Riesgos detectados en la auditoría inicial y resultado

### 3.1 Cobertura remota incompleta

Resuelto por el commit `2f1f1a8`: los jobs `Signatures and protocols RC`,
`Graph pipeline and exact DAG RC`, `Root workspace quality` y el agregador
`Required gates` ejecutan ya las suites que faltaban.

En la auditoría inicial, `Stable quality gates` ejecutaba Microfield, legado y
tres suites raíz, pero dejaba fuera:

- `microcanon`;
- `graph_signatures_v2`;
- `graph_g12` y `graph_g13_g14`;
- `rc_supported_surface`;
- `rc_signature_api`;
- `rc_delta_api`;
- `rc_summary_tree`;
- `rc_database_reconciliation`;
- `rc_graph_dag`.

La corrección no las concentró en `Stable quality gates`: las distribuyó entre
jobs dedicados y las incorporó al agregador `Required gates`. El run post-merge
`30910486012` confirma los 13 jobs verdes. Miri y ASan siguen siendo gates
generales; RC.7 debe ampliar la presión adversarial específica sobre wires,
journals, reconciliación y DAG.

### 3.1.1 Bloqueo reproducido de F6.V

Resuelto antes de la PR: el artefacto semántico se regeneró, el segundo diff
quedó vacío y F6.V pasó en x86-64 y AArch64 tanto en el candidato como después
del merge.

En el run histórico `30906168149`, los jobs `F6.V reproducibility (x86_64)` y
`(aarch64)` fallaron porque la regeneración modificaba una línea de
`validation/f6/results/semantic-v1.json`:

```text
ValidatedPrimitive: ... public API ... pending
→
MaintainedPrimitive: public bounded set recovery ... v1 rejects multiplicity
```

El cambio era coherente con RC.5 y se reproducía desde el clon limpio. No era
una divergencia entre arquitecturas ni un fallo del decoder: el artefacto
semántico versionado había quedado desactualizado al promover reconciliación a
API pública. Se regeneró, revisó y reprodujo con segundo diff vacío; por eso el
SHA histórico `c22323f` fue sustituido por el candidato integrable `2f1f1a8`.

### 3.2 `main` sin gobernanza

La integración formal y el check agregado ya existen. La protección/ruleset de
la rama no se revalidó en esta auditoría y continúa como control de gobernanza
que debe comprobarse antes de una RC final.

En la auditoría inicial no había rulesets, required checks, PR de integración
ni tag. La PR, el tag y `Required gates` ya existen; solo falta volver a
verificar la protección/ruleset efectiva de `main`.

### 3.3 Workflows aún no activos desde default

Resuelto por el merge `d0f4fcd`: los tres workflows están presentes en
`main`.

Antes del merge, `f6-external-corpus.yml` y `microfield-calibration.yml` solo
estaban en la rama candidata y GitHub no podía registrarlos de forma estable.
El merge resolvió esa condición; las próximas ejecuciones programada y manual
deben conservarse como evidencia operativa continua, no como bloqueo de la
integración ya realizada.

### 3.4 Peso histórico

El repositorio local ocupa aproximadamente 459 MiB de `.git`. Los primeros commits
incluyeron artefactos de `target/`; ya no están en el tree actual, pero siguen
en la historia. El tree conserva además datos grandes, incluido
`data/chemistry/results/hts_1m_cache.bin` de unos 83 MiB.

Esto no ha perdido contenido y no impide compilar, pero aumenta latencia,
transferencia y coste de runners. No se reescribirá historia durante la
integración: hacerlo invalidaría hashes, enlaces a Actions y evidencia. Una
limpieza futura necesitará bundle/mirror verificable y ventana específica.

### 3.5 Paquete y metadatos

`cargo package -p homomorphic-hash-rs --list --allow-dirty` enumera hoy 257
rutas e incluye los 20 archivos de `data/` (unos 104 MiB). También avisa de que
faltan licencia, repository, homepage y documentation. Esto no bloquea el
consumo Git interno, pero sí una publicación externa razonable: RC.9 debe
decidir si excluir, separar o versionar explícitamente esos datasets, y la fase
de publicación debe completar los metadatos.

### 3.6 Rendimiento sobre runners compartidos

Los runners GitHub-hosted no ofrecen CPU, frecuencia ni ruido constantes. Son
válidos para detectar fallos funcionales y regresiones catastróficas, no para
prometer ratios pequeños. Los claims de rendimiento necesitarán hardware
dedicado o perfiles agrupados por CPU exacta.

## 4. Invariantes de integración

- No usar squash ni rebase para integrar los 18 commits auditados.
- El candidato final `2f1f1a8` debe quedar como ancestro de `main`.
- El tree de la integración debe ser idéntico al tree candidato salvo los
  commits posteriores dedicados exclusivamente a CI/planificación.
- Antes de integrar se registrará el SHA exacto del nuevo candidato y todos sus
  checks.
- Después de integrar, `git diff <candidate>..main` debe estar vacío para el
  código auditado, o contener únicamente cambios CI previamente revisados.
- `Cargo.lock` se usa siempre con `--locked` en CI.
- Ninguna salida de benchmark modifica un artefacto semántico mantenido.
- Ningún test dependiente de red se mezcla con los gates herméticos de PR.
- Ninguna igualdad de firma rápida se interpreta como identidad exacta.
- No borrar ramas ni objetos hasta crear tag, bundle y manifest de hashes.

## 5. Secuencia de integración y resultado

GI.1, GI.3 y GI.4 están comprobados. GI.0 dejó candidato y checkpoint
reproducibles, aunque cualquier bundle almacenado fuera del repositorio no se
revalidó aquí. La configuración efectiva de GI.2 debe volver a comprobarse en
GitHub antes del go/no-go RC.10.

### GI.0 — congelar y manifestar — ejecutado; backup externo no revalidado

Entregables:

- manifest JSON con SHA commit/tree, `Cargo.lock`, toolchains, lista de refs y
  checks esperados;
- `git bundle --all` local con SHA-256 almacenado fuera del repositorio;
- inventario de archivos grandes y datasets con licencia/origen/hash;
- confirmación final de worktree limpio y remote tracking exacto.
- regeneración F6.V tras RC.5 y segunda ejecución con diff vacío en x86/ARM.

Gate: el bundle se clona y reproduce el mismo tree; ningún ref contiene commits
fuera del candidato.

### GI.1 — completar CI bloqueante en la rama candidata — completado

Separar el workflow rápido en jobs observables:

1. `quality`: fmt, workspace Clippy, rustdoc y `git diff --check`;
2. `workspace`: `cargo test --workspace --all-features --all-targets --locked`;
3. `fields`: matrices no_std/alloc/std, generated/static/runtime y artefactos;
4. `signatures`: leyes, snapshots, deltas, árbol, DB y reconciliación;
5. `graphs`: fast, v2, G12, G13/G14, Microcanon y DAG RC.6;
6. `legacy`: regresión y compatibilidad byte/resultado;
7. `msrv`: superficie compatible con Rust 1.89;
8. `x86-asan` y `arm64`: fronteras ISA y fallback portable;
9. `required-gates`: job agregado que depende de todos los anteriores.

Los tests se distribuirán para evitar ejecutar dos veces toda la suite dentro
del mismo run. Los benches se compilarán, pero no se medirán en PR.

Gate: todos los jobs pasan sobre un clon GitHub y `required-gates` no puede
quedar verde si un job fue omitido, cancelado o falló.

### GI.2 — proteger `main` — estado remoto pendiente de reauditoría

Crear un ruleset con:

- PR obligatoria;
- `required-gates` obligatorio y actualizado con la rama;
- bloqueo de force-push y borrado;
- resolución de conversaciones;
- historial de commits preservado mediante merge commit;
- permisos mínimos `contents: read` para CI;
- Actions externas fijadas por SHA de commit, no solo por tag mutable.

Gate: una PR de prueba no puede integrarse con un check fallido o ausente.

### GI.3 — PR única de integración — completado

- base exacta: `main`;
- head exacta: `agent/h2-5-verified-profiles-pmull` o su sucesora CI;
- adjuntar manifest GI.0 y matriz de 17 commits;
- usar merge commit, nunca squash/rebase;
- comprobar que el candidato es ancestro del merge y que el tree de código es
  idéntico;
- esperar también el run por `push` sobre `main`.

Gate: `main` contiene todo el candidato, no contiene commits divergentes y el
run post-merge es verde.

### GI.4 — checkpoint recuperable — tag completado

Crear tag anotado provisional `internal-rc6-integrated` sobre el commit de
`main`, junto con:

- manifest de integración;
- checksums de corpus y artefactos;
- enlaces a runs y artifacts;
- bundle/mirror de recuperación.

Las ramas históricas se conservan hasta terminar RC.7. Después podrán
archivarse o borrarse solo si cada tip es alcanzable desde el tag.

## 6. Arquitectura de validación remota

### Nivel A — cada PR, bloqueante

Objetivo: 10–20 minutos, hermético y determinista.

- formato, Clippy y rustdoc workspace;
- compile-fail y feature matrix;
- todos los tests no ignorados;
- static/runtime/generated equivalence;
- no_std y MSRV;
- regeneración con diff vacío;
- allocations, unsafe inventory y auditoría de ensamblado;
- x86 y AArch64 reales;
- parsers contra truncación/corrupción en corpus corto;
- clon consumidor externo.

### Nivel B — nightly, bloquea el candidato RC pero no cada PR

Objetivo: hasta 90 minutos por shard.

- exhaustivo de grafos simples n=6;
- campañas aleatorias largas con semillas publicadas;
- reconciliación exhaustiva y logs DB extensos;
- edits de árboles/archivos con tamaños hasta GiB lógicos;
- Miri de firmas, deltas, snapshots, parsers y portable graph core;
- ASan/UBSan en root, no solo Microfield;
- tests paralelos y determinismo a distintos números de threads;
- corpus externo pinned en modo offline tras verificar hashes;
- replay desde snapshots producidos por la versión anterior.

Cada shard subirá JSON/JUnit, seed, toolchain y checksum como artifact.

### Nivel C — semanal/adversarial

Objetivo: varias horas, fail-closed y con oráculos independientes.

- SageMath para campos binarios/primos, firmas y degeneración;
- nauty/Traces para canonización diferencial;
- CFI, SRG, grafos regulares, hipergrafos y multigrafos etiquetados;
- corpus químicos, redes SNAP y datasets XGI pinned;
- límites crecientes de ciclos/walks y presupuestos de Microcanon;
- restauración cruzada de todos los wires `MFSG`, `MFDE`, `MFDJ`, `MFST`,
  `MFRW`, `MFTX`, `MFTL`, `MFRS` y `MFGD`;
- campañas metamórficas multi-campo y static/runtime;
- verificación de que todo `Inconclusive` conserva estado y no se convierte en
  igualdad.

Sage se ejecutará en una imagen/container fijado por digest o en un entorno
Conda reproducible equivalente a `laboratorio_np`; la versión formará parte del
artefacto.

### Nivel D — fuzzing continuo

Targets separados para:

- manifests binarios y primos;
- encoders y canonical decoding;
- wires y journals de firmas;
- filas/transacciones/reconciliación;
- builder de grafos, `GraphDelta`, mapping y canonical documents;
- snapshot `MFGD` y dependencias DAG;
- paridad reference/optimized en reducción, firmas y Microcanon.

PR ejecutará smoke fuzz determinista; nightly 15–30 minutos por target; semanal
varias horas con corpus persistido y minimización automática. Un crash,
divergencia o mutación tras error bloquea RC.7.

## 7. Rendimiento remoto

### 7.1 Qué medir

- campos: scalar, batch, pack/unpack, dispatch y backends ISA;
- firmas: ingest, combine, snapshot, restore y delta por ley/K/campo;
- archivos/árbol: edit local frente a rebuild para 4 KiB–1 GiB lógicos;
- DB: filas/s, transacción, partición, replay y reconciliación por diferencia;
- grafos: filtrado por nivel, Microcanon, paired matcher, delta y DAG;
- memoria, asignaciones, bytes procesados y nodos exactos además de tiempo.

### 7.2 Dos clases de runners

1. GitHub-hosted: guardrail amplio contra regresiones catastróficas, artifacts
   comparables solo cuando CPU/imagen coinciden.
2. Hardware dedicado/self-hosted: claims estables por CPU, microcódigo,
   governor, NUMA, temperatura, toolchain y flags congelados.

No se bloqueará una PR por una variación pequeña en un runner compartido. El
gate inicial será estadístico y conservador; por ejemplo, fallo solo ante una
regresión repetida mayor del 20–30 % con intervalo no solapado. El umbral del
3 % se reserva a calibración dedicada y repetible.

### 7.3 Artefacto de benchmark

Cada ejecución conservará:

```text
commit/tree/lock hashes
runner image y kernel
CPU, flags ISA y microcódigo
rustc/LLVM y RUSTFLAGS
warm-up, samples y confidence interval
seed, dataset y profile IDs
median, MAD, p95, throughput, allocations y peak RSS
baseline comparada y decisión
```

Los resultados se subirán como artifacts y resumen de Actions. Solo los
baselines aprobados se versionarán; un run ruidoso nunca reescribirá la tabla de
selección automática.

## 8. Datos, caché y coste

- `target/` seguirá ignorado y se añadirá un gate que rechace build artifacts
  versionados.
- Los datasets grandes actuales se inventariarán antes de moverlos.
- Nuevos corpus pesados vivirán como release assets o almacenamiento externo
  inmutable con URL, tamaño, licencia y SHA-256.
- GitHub cache se limitará a registry/git/build por lockfile, toolchain, target
  y feature shard; nunca se usará como fuente de verdad.
- PR duplicadas se cancelarán con `concurrency`; nightly/semanal no se
  cancelarán por pushes posteriores.
- Artifacts rápidos: 14–30 días; RC/adversarial: 90 días o release adjunta.
- La limpieza de los 442 MiB históricos queda fuera de la integración. Si se
  aborda, requerirá backup bundle, mirror, aviso de rewrite y comprobación de
  equivalencia de todos los trees relevantes.

## 9. Definition of Done de esta integración

La integración GitHub estará cerrada solo cuando:

1. el run remoto del candidato completo sea verde;
2. cada ref histórica sea ancestro del candidato o esté documentada;
3. `main` se proteja y la PR conserve el historial;
4. todas las suites RC/G12–G14 sean required checks;
5. el run post-merge de `main` sea verde;
6. exista tag, manifest, checksums y bundle recuperable;
7. los workflows manuales/scheduled sean visibles y se ejecuten al menos una
   vez desde `main`;
8. corpus y oráculos estén pinned y funcionen offline tras el fetch;
9. performance separe ruido hosted de claims sobre hardware dedicado;
10. no haya cambios de tree, refs no alcanzables ni archivos locales necesarios
    para construir.

La integración dejó la base remota necesaria para iniciar RC.7. RC.7 añade las
campañas pesadas descritas, pero no vuelve a resolver deuda básica de ramas,
reproducibilidad o cobertura CI; la protección de `main` debe reauditarse como
control de gobernanza.
