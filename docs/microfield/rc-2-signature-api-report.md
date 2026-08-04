# Informe RC.2 — API soportada de firmas

Fecha: 4 de agosto de 2026.

Estado: completado localmente.

## Resultado

Las firmas disponen ya de una fachada pública independiente de grafos y
legado, builders tipados para campos estáticos/runtime, introspección uniforme
y dos formatos de persistencia semánticamente distintos.

No se ha añadido dispatch dinámico al camino algebraico. Cada método del
builder devuelve el tipo concreto que representa su ley.

## Construcción

`SignatureBuilder<F, E>` construye:

- aditiva;
- secuencia;
- secuencia bidireccional;
- multiconjunto;
- multiconjunto multievaluado;
- secuencia multievaluada;
- secuencia y multiconjunto rastreados.

`DynamicSignatureBuilder<E>` ofrece las seis familias compactas sobre un
`DynField` validado sin habilitar el módulo de grafos.

Ambos publican `SignatureFieldProfile`, que conserva `FieldId`, característica
dos, grado, anchura canónica y binding static/runtime. No selecciona campos por
nombre ni oculta la característica.

## Perfiles

`CompactSignature` es un trait público sellado y no object-safe. Permite leer
`SignatureProfile` y serializar el snapshot compacto sin borrar el tipo de la
operación. El perfil contiene:

- `SignatureContext` completo;
- `SignatureAssurance`;
- cardinalidad/longitud;
- número de evaluaciones.

`SignatureEvaluationProfile` congela K=1, K=2 y K=4 como perfiles mantenidos.
Otros valores const-genéricos siguen disponibles, pero permanecen
experimentales hasta medición y admisión explícita.

## Persistencia

### Compacta

`CompactSignature::to_compact_snapshot` produce exactamente el wire `MFSG`
existente. No contiene valores originales y no puede restaurar tracking.

### Exacta rastreada

`TrackedSequence` y `TrackedMultiset` publican snapshots `MFTS` schema 1. El
formato incluye el `MFSG` compacto y los elementos originales con framing y
multiplicidades.

La restauración:

1. valida magic, schema, kind, reserved, longitudes y trailing bytes;
2. aplica `TrackedSnapshotLimits` antes de reservar o expandir;
3. restaura el `MFSG` bajo encoder/base/offset explícitos;
4. reconstruye la colección exacta desde los elementos retenidos;
5. exige igualdad con el estado compacto embebido.

Un error no publica ningún objeto parcial. Los snapshots de secuencia y
multiconjunto no son intercambiables.

## Evidencia

`tests/rc_signature_api.rs` cubre:

- seis familias construidas desde una única fachada estática;
- perfiles K y semántica de característica dos;
- campo primo, binario mantenido y GF(2⁹) externo generado;
- builder runtime con el mismo `FieldId` que el generado;
- equivalencia del snapshot compacto con `MFSG`;
- round-trip exacto de orden y multiplicidad;
- rechazo de truncación en cada byte, trailing data, tampering, base incorrecta
  y kind incorrecto;
- límites de cardinalidad sin mutación;
- determinismo del snapshot multiset ante distinto orden de inserción;
- 500 operaciones aleatorias con checkpoint/rebuild cada 17 pasos.

Gates locales:

| Gate | Resultado |
|---|---|
| RC.2 con `signatures` | 6 tests estáticos |
| RC.2 con `dynamic-signatures`/all-features | 7 tests |
| suite histórica `structural_signatures` | 32/32 |
| Clippy de lib y RC.2 en matrices de features | sin warnings |
| `cargo test -p homomorphic-hash-rs --all-features --all-targets` | 450 unitarios, integraciones, ejemplos y benchmarks sin fallos; 5 tests externos ignorados por diseño |
| Rustdoc con warnings denegados | sin warnings |

## Límites conservados

- las firmas compactas no prueban igualdad de datos;
- `MFTS` es exacto porque retiene datos, con memoria O(n);
- no existen todavía variantes `Tracked*` runtime;
- el encoder multicanal no aporta garantía criptográfica;
- el delta persistible y revisionado quedó incorporado posteriormente en RC.3;
- los profiles K no eligen automáticamente un campo sin conocer el workload.

## Decisión

RC.2 queda cerrado localmente. RC.3 puede construir deltas sobre
`SignatureContext`, snapshots separados y builders públicos sin depender de
grafos o del legado.
