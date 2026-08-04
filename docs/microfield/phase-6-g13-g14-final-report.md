# Informe final F6.G13–G14

Fecha: 3 de agosto de 2026.

## Resultado

G13 y G14 quedan implementados y validados para uso interno. Una tubería
configurable descarta pares distintos tan pronto como existe un witness y
escala, sin falsos positivos, hasta el matcher exacto. Las actualizaciones
disponen de transacción tipada, control de revisión, estimador y fallback.

## G13: filtrado con granularidad

| Tier | Coste dominante | Garantía al diferir | Igualdad |
|---|---:|---|---|
| Metadata | O(1) | exacta | continúa |
| Degree | O(V+I) | exacta | continúa |
| FieldRefinement | O(KR(V+I)) | invariante finito | continúa |
| Patterns | combinatorio acotado | catálogo exacto | continúa |
| LocalPairRefinement | O(r·a³) | 2-WL localizado completo | continúa |
| Exact | IR pareado | decisión exacta | mapping o inconcluso |

`AdaptiveFilterPolicy` expone techo, presupuestos de patterns/pares y budget
exacto. Un techo anterior a exacto produce `Inconclusive`. Cada ejecución
entrega ruta, tiempo, trabajo estimado y skips.

`LocalPairRefinementProfile` detecta celdas ambiguas con descriptores exactos,
añade su frontera de un salto y refina pares solo allí. El preflight `a³·r`
evita trabajos que no caben. Sus bytes `MF2W` son invariantes a renumeración.

## G14: actualizaciones verificadas

`GraphDelta` admite sustitución de label, alta/baja de relación dirigida con
role y multiplicidad y revisión esperada. La aplicación es atómica.
`GraphDeltaPolicy` selecciona `NoChange`, `IncrementalCone` o `FullRebuild`.
El reporte declara endpoints, invalidaciones, filas auditadas y revisión.

Los deltas solo de labels conservan el CSR. La ruta topológica aún construye un
CSR candidato para preservar normalización, aunque evita auditoría y propagación
global si el cono es pequeño. Es un límite explícito del layout actual.

## Evidencia

`graph_g13_g14` cubre rechazo temprano, C6/2C3, relabeling exacto, ceilings,
skips, invariancia MF2W, equivalencia diferencial, fallback, revisión obsoleta
y rollback. Se conservan las pruebas incrementales aleatorias multi-campo y de
role/dirección/loops/multiplicidad.

La campaña se ejecutó dos veces con diff vacío:

```text
SHA-256 0146898c85c064ec396ddd92d8e18409d3450b60983b8c7f88cf4dfafb63de67
wrong_decisions = 0
verified_isomorphisms = 1
differential_updates = 2
maximum_local_audited_vertices = 1
```

Medición release local, n=1.024:

| Ruta | Mediana aproximada |
|---|---:|
| rechazo por Degree (dos grafos) | 126,5 µs |
| delta local de label | 191,9 µs |
| reconstrucción completa | 532,8 µs |

El delta fue 2,78× más rápido. Son datos de una máquina, no un claim portable.
El prefiltro de grados evita deliberadamente la huella de campo hasta que los
cinco histogramas exactos coinciden; esta separación redujo esa ruta un 78,6 %.

## Cierre y límites

G13/G14 cierran el alcance requerido para consumo interno: firmas, filtrado por
niveles, autoridad exacta y actualización transaccional. Quedan separados para
publicación las campañas multi-CPU, nauty/Traces y adapters científicos. La
igualdad no exacta sigue significando `Indistinguishable`.
