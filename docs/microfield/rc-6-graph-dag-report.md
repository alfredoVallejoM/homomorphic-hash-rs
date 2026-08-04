# Informe RC.6 — persistencia canónica, DAG y adapters de grafos

Fecha: 4 de agosto de 2026.

Estado: completado localmente.

## Resultado

RC.6 convierte el canon exacto ya existente en una capacidad persistente para
deduplicar cliques y subredes. La autoridad sigue siendo `Microcanon`; las
firmas de campos finitos, histogramas, patterns y digests solo pueden reducir
candidatos o demostrar diferencias.

El flujo publicado por `CanonicalGraphDag` es:

```text
lookup por metadatos exactos baratos
       │
       ▼
filtro negativo de candidatos
       │
       ▼
Microcanon exacto ── Inconclusive ──► sin mutación
       │
       ▼
bucket por CanonicalGraphKey
       │
       ▼
comparación de canonical_bytes completos
       │
       ├── iguales ─► reuse
       └── distintos ► insert
```

Ni `CanonicalGraphKey` —un índice SHA-256— ni una firma rápida autorizan la
reutilización. Incluso dentro del mismo bucket se comparan todos los bytes.

## DAG exacto y transacciones

Cada `GraphDagNode` conserva:

- `GraphDagNodeId` estable en su linaje de snapshot;
- `CanonicalGraphKey` como índice no autoritativo;
- envelope canónico completo `MFC2`;
- dependencias ordenadas, únicas y ya publicadas.

Esta última precondición construye el DAG en orden topológico y hace imposible
introducir ciclos. Las dependencias forman parte de la descomposición
persistida: intentar reutilizar los mismos bytes con otra descomposición
devuelve `GraphDagDependencyMismatch` en vez de fusionar semántica.

La operación valida schema, revisión optimista, dependencias, búsqueda exacta,
wire y siguiente revisión antes de modificar índices o nodos. Un error o un
presupuesto agotado conserva el estado byte por byte.

## Persistencia `MFGD` v1

El snapshot autocontenido incluye:

- versión de wire;
- `GraphSchemaId`;
- versión productora de la biblioteca;
- revisión y cantidad de nodos;
- IDs, dependencias y bytes canónicos completos.

La restauración aplica límites de bytes, nodos, dependencias y tamaño por nodo;
rechaza truncación, trailing bytes, UTF-8 inválido, referencias futuras,
dependencias desordenadas y schemas incompatibles. Además, decodifica y vuelve
a ejecutar `Microcanon` sobre cada nodo. Solo publica el DAG si la nueva forma
exacta coincide byte a byte con el snapshot.

Esto permite detectar corrupción y también evita aceptar como canónico un
documento meramente normalizado que un emisor externo hubiera ordenado de otra
forma.

## Adapters sin pérdida silenciosa

`GraphSubnetworkAdapter` ofrece tres contratos separados:

- `induced`: conserva kinds, labels, direcciones, relation/role y
  multiplicidades internas; eliminar incidencias de frontera es una elección
  explícita del método;
- `closed`: exige que no exista ninguna incidencia entrante o saliente que
  cruce la selección;
- `relational_clique`: exige entidades y una relación/role dirigida para todo
  par ordenado, y conserva también cualquier otro arco interno.

Se rechazan vertices repetidos, fuera de rango, auxiliares usados como entidades
de clique, fronteras abiertas y relaciones de clique ausentes. No se aplana un
hipergrafo ni se convierte dirección o multiplicidad implícitamente.

## Deltas y política de refresco

G14 invalidaba correctamente todos los canales ante labels o topología, pero
no exponía la causa. `GraphDeltaUpdateReport` publica ahora
`label_changed()` y `topology_changed()` por separado.

`resolve_after_delta` conserva esa decisión como `GraphDagUpdateKind` para
observabilidad. La política de corrección es deliberadamente uniforme:

- no-op: puede terminar en reuse, pero vuelve a pasar por la autoridad exacta;
- label-only: las capas rápidas pueden actualizarse localmente, pero la clave
  persistente se recanoniza;
- topología: recanonización exacta obligatoria;
- cambio mixto: recanonización exacta obligatoria.

Por tanto, la optimización incremental reduce el coste de análisis y filtrado,
pero nunca degrada el gate de identidad.

## Evidencia específica

`tests/rc_graph_dag.rs` cubre:

- reutilización de un grafo bajo permutación tras una comparación exacta;
- rechazo de reutilización entre `C6` y `C3 + C3`, colisión regular clásica;
- dependencias deduplicadas, orden topológico e inmutabilidad ante error;
- conflicto de revisión y de descomposición;
- round-trip persistente con recanonización independiente;
- rechazo de todos los prefijos truncados, corrupción y límites;
- conservación de labels, roles, direcciones y multiplicidades;
- rechazo de fronteras silenciosas y selecciones duplicadas;
- clique dirigido completo y conservación de arcos internos adicionales;
- integración label-only entre `GraphDelta` y refresco exacto del DAG.

`tests/graph_g13_g14.rs` añade gates explícitos para distinguir no-op,
label-only y topología.

## Límites conservados

- `MicrocanonOutcome::Inconclusive` no crea nodo: el consumidor debe aumentar
  presupuesto, diferir o usar otra política;
- el snapshot no es autenticado; la recanonización prueba consistencia exacta,
  no procedencia;
- el adapter de clique v1 valida una relación dirigida concreta, no infiere la
  semántica científica de “clique”;
- los cambios topológicos conservan corrección, pero no se promete todavía una
  mejora incremental end-to-end sobre CSR;
- las firmas homomórficas continúan siendo no criptográficas y no sustituyen
  los bytes exactos.

## Decisión

RC.6 queda cerrado. El vertical de grafos ya dispone de la frontera necesaria
para consumo interno: filtro rápido, autoridad exacta, persistencia versionada,
deduplicación transaccional y adapters explícitos. RC.7 puede concentrarse en
campañas exhaustivas, adversariales, fuzzing, SLO y el artefacto final go/no-go.
