# Informe RC.3 — núcleo versionado de deltas

Fecha: 4 de agosto de 2026.

Estado: completado localmente.

## Resultado

La API mantenida de firmas dispone de un protocolo general de cambios
incrementales independiente de grafos. Los deltas son tipos distintos por ley;
no existe un payload universal que pueda aplicar una inversa inválida.

La implementación funciona sobre cualquier campo estático que satisfaga los
traits de la firma, incluidos campos externos generados. Los campos runtime
continúan fuera de este contrato de deltas hasta congelar su persistencia de
estado y configuración.

## Contrato transaccional

`RevisionedSignature<S>` retiene namespace, estado compacto, revisión y los
`DeltaId` ya comprometidos. La aplicación sigue siempre:

1. validar namespace y `SignatureContext`;
2. reconocer replay exacto por `DeltaId`;
3. comprobar la revisión de origen;
4. calcular y validar un candidato sin mutar el estado publicado;
5. publicar candidato, revisión e identidad comprometida.

Un error conserva estado, revisión y conjunto de replay. Cada transacción
válida avanza exactamente una revisión; los deltas vacíos y el overflow se
rechazan.

## Tipos por ley

- `AdditiveDelta`: resta una partición asumida y suma otra, ajustando el
  contador de términos;
- `MultisetDelta`: divide factores no cero, contabiliza ceros y añade la nueva
  partición;
- `SequenceAppend`: concatena una firma de sufijo;
- `SequenceTrim`: recupera algebraicamente el prefijo a partir de un sufijo
  asumido.

La retirada en un estado compacto no demuestra pertenencia ni orden real. El
resultado se clasifica como `DeltaVerification::AlgebraicConsistency`. Los
niveles `SourceValidated` y `ExactRebuild` existen para adapters que aporten una
fuente autoritativa o reconstrucción exacta; RC.3 no los afirma por defecto.

## Persistencia

Cada delta usa el formato determinista `MFDE` schema 1 con:

- kind de ley;
- `ApplicationNamespace`;
- revisión origen/destino;
- número de operaciones;
- `FieldId`, `EncoderId`, `SignatureId` y ley;
- payload de firmas `MFSG` con longitudes explícitas.

`DeltaId` deriva del envelope y payload completos. Es identidad de contenido,
no autenticación.

`DeltaJournal<D>` usa `MFDJ` schema 1. Exige una cadena contigua, una sola
identidad, ausencia de duplicados y framing completo. `DeltaJournalLimits`
limita entradas, bytes por entrada y bytes totales antes de reservar memoria.
El replay completo opera sobre un candidato y solo se publica si todas las
entradas terminan correctamente. Un segundo replay sobre el mismo estado omite
las transacciones ya publicadas.

## Evidencia

`tests/rc_delta_api.rs` incorpora:

- 400 deltas aditivos aleatorios, comparados con rebuild tras cada revisión;
- persistencia, restauración y doble replay del journal completo;
- 300 deltas de multiconjunto con duplicados y factores cero;
- 80 appends y 40 trims ordenados frente a rebuild exacto;
- namespace incorrecto, revisión obsoleta y retirada con underflow;
- comprobación byte a byte de que los fallos dejan el estado intacto;
- rechazo de todos los prefijos truncados de `MFDE` y `MFDJ`;
- rechazo de magic, schema, kind, reserved, revisiones, contadores e identidades
  corruptos;
- límites de journal, duplicación, gaps y reordenación;
- rollback completo cuando una entrada intermedia del journal falla.

Gates locales:

| Gate | Resultado |
|---|---|
| RC.3 con `signatures` | 8/8 |
| RC.3 con `--all-features` | 8/8 |
| campaña incremental | 400 aditivos + 300 multiconjunto + 120 secuencia |
| campo externo generado | GF(2⁹), delta/wire/apply correctos |
| suite raíz `--all-features --lib --tests` | 450 unitarios e integraciones sin fallos; 5 ignorados por diseño |
| `--all-features --all-targets` | ejemplos y benchmarks incluidos, sin fallos |
| Clippy | sin warnings |
| Rustdoc con warnings denegados | sin warnings |
| formato y `git diff --check` | correctos |

## Límites conservados

- los deltas compactos no prueban que un elemento retirado existiera;
- `DeltaId` detecta replay idéntico, no manipulación maliciosa;
- el estado revisionado todavía no tiene snapshot propio: RC.4 combinará
  checkpoint y journal para recuperación tras restart;
- append/trim cubren los extremos; edits de rango requieren el árbol de RC.4;
- los deltas runtime y multi-evaluación permanecen fuera de la superficie
  soportada;
- no se ofrece rollback genérico: debe modelarse como otro delta autorizado.

## Decisión

RC.3 queda cerrado. RC.4 puede construir adapters de chunks y un árbol
jerárquico sobre `SequenceAppend`, firmas compactas, snapshots y journals sin
introducir semántica de archivos en el núcleo algebraico.
