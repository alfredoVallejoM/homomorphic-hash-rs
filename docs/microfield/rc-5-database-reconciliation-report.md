# Informe RC.5 — base de datos y reconciliación

Fecha: 4 de agosto de 2026.

Estado: completado localmente.

## Resultado

RC.5 incorpora dos protocolos mantenidos e independientes de grafos:

1. filas canónicas y transacciones particionadas;
2. reconciliación acotada de conjuntos sobre Fp251.

Ambos permanecen detrás de `signatures` y no necesitan activar legado ni
canonización.

## Filas y schema

`DatabaseSchema` congela versión, columnas ordenadas, tipos, nullabilidad y
columnas de clave primaria. `DatabaseSchemaId` deriva de todo el descriptor.

V1 admite valores:

- `Bool`;
- `I64` y `U64` little-endian;
- `Bytes` con longitud;
- `Text` UTF-8 exacto, sin normalización ni collation implícita;
- `Null` solo en columnas declaradas nullable.

El wire `MFRW` incluye schema, versión exacta de fila y todos los valores. La
clave primaria canónica excluye la versión, permitiendo identificar un update;
la versión sí participa en la firma de la imagen completa.

## Tabla particionada y transacciones

`PartitionedDatabase<F, E>` mantiene:

- namespace de aplicación;
- schema;
- revisión global;
- mapa exacto de filas por partición;
- firma multiconjunto por partición;
- IDs de transacciones ya comprometidas.

La partición deriva de `DatabaseRowKey`. Una transacción clona únicamente las
particiones afectadas, valida todas las mutaciones sobre candidatos y publica
un solo commit. No clona la tabla completa durante el hot path normal.

`TransactionDelta` `MFTX` schema 1 ofrece:

- insert con clave ausente;
- delete con before image exacta;
- update con before/after, misma clave y versión creciente;
- namespace, schema, revisiones y `TransactionId` de contenido.

Una clave solo puede aparecer una vez por transacción. Los cambios de clave se
modelan como delete+insert sobre claves distintas. Los conflictos, límites y
fallos de encoder conservan tabla, revisión e historial anteriores.

`DatabaseTransactionLog` usa `MFTL` schema 1, exige cadena contigua y replay
idempotente. El replay completo usa un candidato y no publica prefijos si una
transacción intermedia falla.

### Multiplicidad

La tabla v1 representa una relación con clave primaria única. Insertar una clave
duplicada es error; no se interpreta como una segunda ocurrencia. Una aplicación
que necesite duplicados debe incluir un discriminante estable en la clave.

## Reconciliación mantenida

`BoundedSetReconciler` promueve el decoder antes privado de
`validation-lab`. Usa evaluaciones de polinomios característicos en Fp251 y
publica:

- `ReconciliationLimits` para universo, diferencia, grado y memoria;
- `ReconciliationProfileId`;
- sketch persistible `MFRS`;
- `RecoveredSetDifference` orientado;
- errores tipados para perfil, wire, conjunto, denominador y fuera de cota.

Los miembros deben estar ordenados, ser únicos y pertenecer al universo. V1 es
reconciliación de conjuntos: las multiplicidades se rechazan explícitamente.

El receiver aporta su conjunto exacto. El decoder valida pertenencia de las
retiradas y reconstruye el sketch remoto antes de devolver un candidato.

La cota de diferencia es una precondición del protocolo, no una autenticación.
Una diferencia exterior normalmente produce `DifferenceExceedsBound`, pero un
sketch finito no puede excluir toda colisión adversarial fuera de cota. Dentro
de la cota y los perfiles ensayados, la recuperación es exacta.

## Evidencia

`tests/rc_database_reconciliation.rs` cubre:

- tipos, nullability, UTF-8, versión y clave primaria;
- 400 transacciones aleatorias insert/update/delete;
- comparación con rebuild exacto tras cada commit;
- transacción multi-row atómica;
- conflictos de before image y rollback de log intermedio;
- persistencia y doble replay idempotente;
- rechazo de cada prefijo truncado de `MFTX` y `MFTL`;
- tabla sobre GF(2⁹) externo generado;
- las 63.232 parejas exhaustivas del corpus histórico con distancia ≤6;
- rechazo tipado de las 2.304 parejas restantes fuera de cota en universo 8;
- rechazo tipado de una diferencia exterior de referencia;
- wire `MFRS`, perfiles incompatibles, evaluaciones no canónicas y límites;
- rechazo explícito de miembros duplicados.

El laboratorio ya no conserva un decoder alternativo: su campaña llama a la
API pública mantenida.

Gates locales:

| Gate | Resultado |
|---|---|
| RC.5 con `signatures`/all-features | 8/8 |
| transacciones diferenciales | 400 commits + rebuild por revisión |
| reconciliación exhaustiva | 63.232 recuperadas + 2.304 fuera de cota rechazadas |
| decoder del laboratorio | sustituido por adapter público; tests verdes |
| tabla externa generada | GF(2⁹), transacción y resumen correctos |
| suite raíz `--all-features --all-targets` | 450 unitarios, integraciones, ejemplos y benchmarks sin fallos; 5 ignorados por diseño |
| Clippy raíz + validation-lab | sin warnings |
| Rustdoc con warnings denegados | sin warnings |

## Límites conservados

- no hay adapter SQL ni conexión a un motor externo;
- el consumidor debe mapear sus LSN/offsets a las revisiones de la API;
- `Text` usa igualdad de bytes UTF-8, no collation de base de datos;
- los logs no autentican el origen;
- la reconciliación v1 usa Fp251 y universo menor que el campo;
- no se soportan multiplicidades en reconciliación;
- las firmas de filas siguen siendo fingerprints no criptográficos.

## Decisión

RC.5 queda cerrado. RC.6 puede concentrarse en el schema canónico persistente
de grafos, DAG exacto y adapters de cliques/subredes sin dejar pendientes en el
núcleo de archivos, DB o reconciliación.
