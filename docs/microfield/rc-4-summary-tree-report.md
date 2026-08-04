# Informe RC.4 — archivos y árbol jerárquico

Fecha: 4 de agosto de 2026.

Estado: completado localmente.

## Resultado

La superficie mantenida de firmas dispone ya de un adapter exacto para archivos
divididos en chunks y un árbol jerárquico de resúmenes ordenados. El árbol está
separado de grafos y no introduce I/O dentro del núcleo: opera sobre slices y
checkpoints que el consumidor puede almacenar donde corresponda.

`HomomorphicSummaryTree<F, E>` es genérico sobre cualquier campo estático o
externo generado compatible con la firma de secuencia.

## Identidad y framing

`FileChunkProfile::fixed(n)` congela:

- algoritmo de chunking fijo;
- bytes nominales por chunk;
- tratamiento del último chunk;
- framing de hoja `MFFC` schema 1;
- topología binaria, combinación por pares y promoción del nodo impar.

Su `FileChunkProfileId` forma parte de `HomomorphicSummaryRoot`. Dos árboles con
distinto tamaño de chunk no comparten identidad aunque utilicen el mismo campo,
encoder y base.

Cada hoja codifica magic, schema, longitud exacta y contenido. La secuencia de
hojas se combina mediante:

```text
H(A || B) = H(A) · base^len(B) + H(B)
```

La raíz expone además longitud exacta en bytes y número de chunks. Sigue siendo
un fingerprint algebraico no criptográfico; los chunks retenidos son la fuente
exacta de verdad. Su descriptor portable `MFSR` conserva perfil, contexto,
longitudes y evaluación, evitando persistir una evaluación desnuda.

## Edits y complejidad

Se ofrecen:

- `replace_range`;
- `insert_range`;
- `remove_range`;
- `append`;
- `truncate`.

Un reemplazo con la misma longitud conserva todas las fronteras. Se preparan
las nuevas hojas y sus ancestros fuera del estado publicado, y después se hace
un único commit. Una hoja aislada recomputa exactamente un nodo por nivel:
O(log n). Un rango de k hojas cuesta O(k log n) como cota conservadora y suele
compartir ancestros.

Insertar, retirar, hacer append/truncate o reemplazar por otra longitud cambia
las fronteras del perfil fijo. RC.4 selecciona explícitamente
`BoundaryRebuild`: materializa el candidato completo, lo valida y solo después
lo publica. No intenta presentar esta vía como incremental.

`SummaryEditReport` expone path, hojas tocadas, nodos recomputados y revisión.
Los no-op no avanzan la revisión.

## Persistencia y recuperación

El checkpoint `MFST` schema 1 contiene:

- `FileChunkProfileId`;
- revisión;
- longitud exacta;
- raíz compacta `MFSG`;
- bytes exactos del archivo.

La restauración aplica `SummaryTreeLimits`, reconstruye chunks y todos los
nodos, y exige que la raíz reconstruida coincida con la embebida. Magic, schema,
reserved, perfil, longitudes, trailing bytes y límites se validan antes de
publicar el árbol.

El checkpoint no autentica datos hostiles. Detecta inconsistencia algebraica y
permite recuperación exacta de los bytes que contiene; una aplicación que
necesite autenticación debe aportarla externamente.

## Evidencia

`tests/rc_summary_tree.rs` cubre:

- archivo vacío y cada frontera inmediata de chunk;
- identidad distinta para perfiles de 8 y 16 bytes;
- reemplazo de una hoja con cota logarítmica observada;
- reemplazo que cruza cuatro hojas y comparte ancestros;
- 600 edits aleatorios mezclando replace/insert/remove/append/truncate;
- comparación de bytes y raíz contra rebuild después de cada edit;
- checkpoints después de rutas local y rebuild;
- rechazo de cada prefijo truncado y de trailing bytes;
- perfil, raíz y límites incompatibles sin mutación;
- ejecución sobre un GF(2⁹) externo generado.

`benches/structural_signatures.rs` añade mediciones separadas de edit local y
rebuild completo para 64 KiB y 1 MiB, con chunks de 1 KiB.

Medición local orientativa, build release, 4 de agosto de 2026:

| Archivo | edit local | rebuild | relación observada |
|---:|---:|---:|---:|
| 64 KiB | 24,93 µs | 1,141 ms | 45,8× |
| 1 MiB | 50,57 µs | 18,47 ms | 365× |

El setup clona el árbol fuera de la región medida de Criterion. Estas cifras
demuestran el comportamiento incremental en esta máquina, no constituyen aún
un SLO multi-microarquitectura.

Gates locales:

| Gate | Resultado |
|---|---|
| RC.4 con `signatures` | 6/6 |
| RC.4 con `--all-features` | 6/6 |
| campaña diferencial | 600 edits, cero divergencias |
| checkpoint adversarial | todos los prefijos truncados rechazados |
| campo externo generado | GF(2⁹), build/edit/rebuild correctos |
| suite raíz | 450 unitarios e integraciones sin fallos; 5 ignorados por diseño |
| `--all-features --all-targets` | ejemplos y benchmarks incluidos, sin fallos |
| Clippy y Rustdoc | sin warnings |

## Límites conservados

- los datos se retienen en memoria; streaming desde `Read` y mmap son adapters
  posteriores;
- el chunking v1 es fijo, no content-defined;
- los cambios de longitud reconstruyen todo el árbol;
- `MFST` es checkpoint, no journal ni formato autenticado;
- la raíz no demuestra igualdad exacta de archivos por sí sola;
- no se persisten caches ISA ni representaciones internas.

## Decisión

RC.4 queda cerrado. RC.5 puede construir particiones y transacciones de filas
sobre el núcleo de deltas, mientras el árbol queda disponible como índice
jerárquico general para archivos y colecciones ordenadas.
