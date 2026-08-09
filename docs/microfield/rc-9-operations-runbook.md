# RC.9 — runbook de operación, recuperación y migración

Fecha: 9 de agosto de 2026.

Este runbook cubre el consumo interno de campos, firmas algebraicas
homomórficas no criptográficas, deltas, archivos/árboles, base de datos,
reconciliación y DAG canónico. No convierte una firma compacta en autenticador,
prueba de pertenencia ni autorización de borrado.

## Identidad y telemetría mínima

Cada proceso debe registrar al iniciar y al cambiar de dataset:

- versión de crate, toolchain y arquitectura;
- `FieldId`, backend del `Engine` y features activas;
- `SignatureLaw`, `SignatureAssurance`, `EncoderId` y `SignatureId`;
- `ApplicationNamespace`, schema/profile ID y revisión actual;
- ruta de actualización (`LocalTree`, `BoundaryRebuild`, `Incremental`,
  `AuthoritativeRebuild` o replay idempotente);
- límites aplicados y contador de rechazos por límite, identidad, revisión,
  corrupción o conflicto;
- en grafos, tier ejecutado, presupuesto, fallback y cualquier
  `Inconclusive` completo;
- campaign RC.8, escala, p50/p95/p99, allocations, memoria pico y ratio contra
  rebuild.

No se registrarán los bytes exactos de filas o documentos salvo que la política
de datos de la aplicación lo autorice. Las identidades y revisiones sí deben
estar siempre presentes.

## Publicación durable de artefactos

La API publica bytes canónicos, no durabilidad de filesystem. El consumidor
debe:

1. escribir a un fichero temporal en el mismo filesystem;
2. cerrar y sincronizar el contenido según su modelo de fallos;
3. renombrar atómicamente al nombre versionado;
4. sincronizar el directorio cuando la garantía del sistema lo requiera;
5. publicar después el puntero/checkpoint activo.

Un rename no coordina escritores múltiples. La aplicación debe serializar por
namespace/schema y conservar fencing/lease o transacción externa.

## Revisión, LSN y replay de base de datos

Se asigna exactamente una revisión de biblioteca a cada transacción confirmada
por la fuente autoritativa. El consumidor persiste el mapeo
`(namespace, schema_id, source_revision, target_revision, LSN)` junto al log.
No se sintetizan revisiones para huecos ni se reordena el log.

- `RevisionMismatch`: detener replay, comprobar LSN/checkpoint y reconstruir
  desde la última fuente autoritativa; no forzar la revisión.
- `AlreadyApplied`: tratar como replay idempotente y verificar que el LSN
  asociado sea el mismo.
- `Conflict`: comparar before image con la fila exacta. Si la fuente cambió,
  descartar el delta y regenerarlo.
- `SchemaMismatch` o `NamespaceMismatch`: no reinterpretar bytes; seleccionar
  el reader/migración correcto.
- `LimitExceeded`: rechazar antes de ampliar límites. Revisar tamaño de fila,
  transacción, log y memoria con el SLO correspondiente.

`DatabaseApplyPolicy` usa aplicación incremental hasta el ceiling medido. Por
encima solicita las filas objetivo autoritativas, reconstruye y verifica
exactamente todas las before/after images antes de publicar. Un mismatch deja
estado y revisión intactos.

## Recuperación y reconstrucción

Orden de recuperación:

1. validar header, schema, IDs y límites del último checkpoint;
2. si es válido, cargarlo y reproducir solo el journal/log contiguo posterior;
3. comparar estado final con la fuente exacta o con un rebuild independiente;
4. si el checkpoint o log está corrupto, aislar el artefacto y reconstruir
   desde bytes/filas exactos;
5. emitir un checkpoint nuevo; nunca reparar campos internos a mano.

Para archivos se reconstruye `HomomorphicSummaryTree` desde los bytes exactos.
`SummaryEditPolicy` usa recomposición local solo hasta su ceiling medido. Los
cambios de longitud y edits grandes toman `BoundaryRebuild`.

Para el DAG, `from_canonical_bytes` recanoniza y valida dependencias. Un fallo o
agotamiento de presupuesto no autoriza reutilización: aumentar el presupuesto
de forma controlada o reconstruir desde los grafos exactos.

## Corrupción y cuarentena

Ante un error de wire, checksum estructural, truncación, bytes trailing o
representación no canónica:

- no reintentar el mismo parser con límites relajados;
- copiar el artefacto a cuarentena con hash de transporte, origen y timestamp;
- registrar parser/schema/longitud y clase de error, no datos sensibles;
- recuperar desde la última fuente exacta conocida;
- convertir el caso mínimo no sensible en fixture de regresión.

Las firmas algebraicas pueden colisionar. Su igualdad no sustituye la
comparación exacta al investigar corrupción.

## Migración

Una migración cambia de forma explícita alguno de: campo, encoder, ley,
parámetros, schema, chunk profile o graph schema. Procedimiento:

1. congelar escrituras o capturar un LSN consistente;
2. leer con el decoder antiguo y sus límites;
3. reconstruir desde contenido exacto usando el perfil nuevo;
4. persistir artefactos nuevos con IDs nuevos;
5. ejecutar comparación exacta y, cuando aplique, replay diferencial;
6. cambiar el puntero activo de forma atómica;
7. conservar rollback al checkpoint anterior durante la ventana acordada.

No se cambia un ID dentro de bytes antiguos ni se convierte un residual
algebraico en prueba de migración.

## Reconciliación

V1 opera sobre conjuntos únicos de `u16` y una diferencia máxima declarada. El
receiver set exacto valida la recuperación. Duplicados, multiplicidad, universo
o diferencia fuera de límites son error tipado y requieren protocolo distinto
o transferencia/rebuild exacto.

## Comandos de diagnóstico

```bash
cargo test --workspace --all-features --all-targets --locked
cargo test --manifest-path test-fixtures/rc-consumer/Cargo.toml --all-targets --locked
cargo run --manifest-path test-fixtures/rc-consumer/Cargo.toml --locked -- /tmp/rc9-consumer
cargo run --release -p microfield-validation-lab -- rc8-capacity \
  --manifest validation/rc/capacity-manifest-v1.json --out /tmp/rc8.json
bash tools/audit_rc_package.sh
```

El fixture externo es la referencia ejecutable del ciclo persistir → reiniciar
→ replay → rebuild → corrupción/schema drift. No usa `legacy` ni módulos
privados.
