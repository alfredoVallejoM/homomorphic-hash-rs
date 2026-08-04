# Informe de implementación — F6.G8 y baseline F6.G9

Fecha: 3 de agosto de 2026.

Estado: completado localmente. F6.G10 se cerró posteriormente; véase el
[`informe G10`](phase-6-g10-final-report.md).

## Resultado

La autoridad exacta ya no pertenece a una firma de campo. `Microcanon` recibe
solo un `IncidenceGraph`, un `GraphSchemaId` y un presupuesto. F251,
GF(2^256), el encoder, el número de lanes, las rondas y la estrategia rápida no
aparecen en el encoding exacto ni pueden cambiar su resultado.

La implementación conserva las firmas algebraicas como filtros y descriptores:
una diferencia continúa siendo evidencia útil; una igualdad nunca confirma
isomorfismo. El resultado positivo exacto se publica únicamente después de
recorrer el árbol necesario y verificar de nuevo la correspondencia completa.

## Contratos introducidos

- `GraphSchemaId`: identidad de la semántica de aplicación;
- `GraphAnalysisProfileId`: identidad separada para campos, lanes y políticas;
- `CanonicalGraphEncodingId::V1`: envelope exacto `MFC2`, versionado y
  big-endian;
- `CanonicalGraphForm`: bytes completos, mappings inversos y
  `CanonicalGraphKey` SHA-256 solo como índice;
- `CanonicalGraphDocument`: parser estricto con reconstrucción, normalización,
  rechazo de truncación, trailing bytes, versiones y contadores incoherentes;
- `VerifiedGraphMapping`: bijección revalidada contra kinds, labels, dirección,
  relación, rol y multiplicidad;
- `GraphComparison`: `Different`, `Isomorphic { mapping }` o `Inconclusive`;
- `MicrocanonOutcome`: forma exacta o salida incompleta sin best-so-far.

El modelo añade accesores comprobados para IDs suministrados externamente. La
construcción de hiperaristas valida todos sus endpoints antes de publicar el
vértice auxiliar y prohíbe usar otra hiperarista como entidad participante.

## Encoding exacto v1

```text
MFC2 | encoding_version | model_version | GraphSchemaId
| vertex_count | directed_record_count | total_multiplicity
| (kind, frame(label))*
| (source, target, frame(relation), frame(role), multiplicity)*
```

Los registros dirigidos se ordenan por la tuple semántica completa. El parser
reconstruye el modelo normalizado y exige que volver a codificarlo produzca los
mismos bytes. `CanonicalGraphKey` se calcula sobre el documento completo; la
igualdad autoritativa compara `bytes()`, no solo el digest.

El formato v0 `MFCG`, que incluía `GraphSignatureId`, no se reinterpreta como
v1. Cualquier índice persistido con v0 necesita regeneración explícita.

## Baseline exacto

El núcleo G9 implementa:

1. claves iniciales exactas de kind y label;
2. refinamiento relacional dirigido hasta punto fijo con multisets exactos de
   color vecino, relación, rol y multiplicidad;
3. descomposición segura por componentes débiles;
4. individualización–refinamiento DFS completo;
5. mínimo lexicográfico de todas las hojas exploradas;
6. límites de nodos y celdas retenidas con salida fail-closed;
7. parse/re-encode y verificación lineal del mapping antes de publicar.

`FastGraphLabeler::canonicalize_exact` se mantiene como adapter de
compatibilidad. Ejecuta su diagnóstico de degeneración, pero delega la forma
exacta en `Microcanon`. `try_canonicalize` conserva su condición rápida de
partición discreta y usa Microcanon con presupuesto IR cero para fijar el orden
independiente. `DiscreteCanonicalForm` es un alias de compatibilidad de
`CanonicalGraphForm`.

## Evidencia ejecutada

Pruebas específicas nuevas:

- igualdad byte a byte entre F251/GF(2^256), distintos encoders, lanes y
  perfiles;
- separación de `GraphSchemaId` sin cambiar el grafo reconstruido;
- round-trip completo y rechazo de toda truncación posible del fixture;
- mappings válidos, no biyectivos y semánticamente incorrectos;
- comparación de isomorfos, no isomorfos con los mismos contadores e
  `Inconclusive` por presupuesto cero;
- dirección, roles, multiplicidad e hiperaristas;
- acceso comprobado a IDs y publicación transaccional de hiperaristas;
- equivalencia con las 15 pruebas adversariales G5–G7 y el oráculo previo hasta
  cinco vértices.

Gate exhaustivo explícito:

```text
cargo test -p homomorphic-hash-rs --release --test graph_canonical \
  microcanon_matches_every_simple_graph_isomorphism_class_at_six_vertices \
  -- --ignored --exact
```

Resultado observado:

```text
32.768 grafos simples etiquetados de orden 6
156 clases del oráculo factorial independiente
156 formas Microcanon distintas
0 divergencias dentro de clase
0 colisiones entre clases
```

También quedaron verdes `cargo test --workspace --all-features`, Clippy con
`-D warnings`, formato y `git diff --check`.

## Límites y siguiente hito

G9 es deliberadamente una referencia clara, no el motor industrial final:

- crea claves `Vec<u8>` por vértice/pasada;
- no usa active-cell refinement, arenas planas ni radix/counting sort;
- no extrae automorfismos u órbitas y no poda ramas equivalentes;
- el presupuesto cubre nodos y frontier de colores, no todos los bytes
  temporales, tiempo o profundidad;
- `compare` canoniza ambos grafos; el matcher pareado pertenece a G12;
- todavía no implementa el catálogo de loops ni RGI, que pertenecen a G11.

G10 convirtió posteriormente este baseline en un motor compacto, conservando
`MicrocanonStrategy::Reference` como oráculo diferencial y los mismos bytes
exactos. El detalle y las desviaciones del diseño inicial están en el
[`informe de cierre G10`](phase-6-g10-final-report.md).
