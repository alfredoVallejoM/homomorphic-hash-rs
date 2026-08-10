# C3 D2: PostgreSQL y change stream de sistemas

Fecha: 2026-08-11. Estado: **cinco escenarios D2 implementados y ejecutados
como preflight externo; réplica `Controlled`, otras versiones y soak largo
pendientes**.

Los resúmenes evaluados son algebraicos y **no criptográficos**. Las cifras de
este documento se obtuvieron en un host compartido y sólo sirven para detectar
errores y fronteras; no son benchmarks de publicación.

## Qué se ejecutó

El nuevo binario `c3_systems` inició PostgreSQL 17.10 con
`wal_level=logical`, creó un slot `test_decoding` y ejecutó cinco escalones de
1, 2, 8, 16 y 32 clientes. Cada transacción actualizó una fila bajo lock,
asignó una revisión total y dejó una before/after image. El consumidor asoció
esa revisión al LSN del `COMMIT` decodificado y aplicó la mutación al adaptador
de Algesum.

En total se observaron 1.888 de 1.888 commits. En todos los escalones:

- el número de updates WAL, eventos y transacciones coincidió;
- los LSN de commit fueron estrictamente crecientes;
- cada before-image coincidió con el modelo exacto;
- las 8.192 filas PostgreSQL coincidieron con el espejo;
- el resumen incremental coincidió con una reconstrucción completa.

## Concurrencia y frontera observada

| Clientes | Commits | Tiempo | Throughput diagnóstico |
|---:|---:|---:|---:|
| 1 | 32 | 63,95 ms | 500,4 tx/s |
| 2 | 64 | 81,88 ms | 781,6 tx/s |
| 8 | 256 | 232,04 ms | 1.103,3 tx/s |
| 16 | 512 | 458,39 ms | 1.117,0 tx/s |
| 32 | 1.024 | 1.046,35 ms | 978,6 tx/s |

La mejora se aplana entre 8 y 16 clientes y retrocede un 12,4 % a 32. El
laboratorio serializa una revisión durable en `algesum_c3_metadata`, de modo
que esta caída mide deliberadamente contención de orden total además del coste
de conexiones, locks y WAL. No demuestra todavía el techo de PostgreSQL ni el
de un consumidor de replicación dedicado.

## Backpressure, migración y restart

Sobre el último burst de 1.024 commits se drenaron ventanas equivalentes a 2×
y 5×: 512 filas en dos lecturas y 204 filas en seis lecturas. En ambos casos el
backlog fue monótono, acabó en cero y las revisiones fueron contiguas. La
duración fue 3,61 ms y 2,60 ms respectivamente; son lecturas de catch-up con el
productor detenido, no percentiles de steady state.

La migración compatible añadió `status NOT NULL DEFAULT 'active'`: cubrió las
8.192 filas, mantuvo estable la proyección v1 y produjo un resumen v2
determinista. Una migración incompatible de `balance bigint` a `text` fue
detectada por el decoder tipado y revertida; después del rollback la columna
seguía siendo `bigint`.

Finalmente se persistieron revisión, agregados, versión de schema, número de
particiones y bytes canónicos del resumen. Tras `docker restart`, PostgreSQL
recuperó las 8.192 filas, revisión 1.024 y suma 335.504.384; todos los campos y
bytes coincidieron y una reconstrucción nueva produjo el mismo resumen.

## Lectura correcta y trabajo pendiente

D2 cierra logical decoding, concurrencia, restart y migración como
**preflight**, y sustituye la entrega directa ficticia por un catch-up de
backpressure verificable. No promueve el adaptador a consumidor PostgreSQL de
producción: `test_decoding` se consulta desde SQL y el laboratorio conserva un
event log auxiliar para verificar before/after images.

Antes de publicar resultados hacen falta PostgreSQL en al menos dos versiones,
un consumidor del protocolo de replicación, escenarios writer/reader
separados, crashes en cada frontera commit/checkpoint, bursts con productor
activo, steady states de 15 min y 1 h, soak de 8 h, 64-256 clientes cuando el
host lo permita y réplica en hardware controlado. La evidencia cruda está en
`c3-d2-postgresql-systems-prepare-v1.json` y
`c3-d2-postgresql-systems-restart-v1.json`.
