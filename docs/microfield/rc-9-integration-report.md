# RC.9 — informe de interoperabilidad y operación

Fecha: 9 de agosto de 2026.

Estado: implementación y validación local completas; matriz remota x86-64 y
AArch64 pendiente antes de declarar integración.

## Consumidor independiente

`test-fixtures/rc-consumer` es un workspace Cargo separado. Depende del paquete
raíz con `default-features = false` y únicamente `signatures,graph`; no puede
apoyarse accidentalmente en `legacy` ni en módulos privados.

El escenario ejecutable cubre:

- firma y delta sobre un campo externo generado;
- persistencia `MFSG`/`MFDJ`, restart, replay y rebuild exacto;
- árbol `MFST`, recomposición local y fallback gobernado;
- row store propiedad del consumidor usando `DatabaseSchema::encode_row`, log
  `MFTL`, replay, rebuild autoritativo y migración real a schema v2;
- rechazo de corrupción y schema drift;
- reconciliación exacta dentro de cota;
- DAG `MFGD`, restart, recanonización y reutilización por bytes exactos;
- observabilidad de backend, revisiones, rutas, fallback e outcomes.

La ejecución local generó siete artefactos persistentes y devolvió revisiones
y rutas esperadas (`LocalTree`, `BoundaryRebuild`, `AuthoritativeRebuild`, DAG
reutilizado), sin acceso a representación privada.

## Operación

[`rc-9-operations-runbook.md`](rc-9-operations-runbook.md) fija telemetría,
publicación durable a cargo del consumidor, mapeo LSN/revisión, respuesta a
errores, cuarentena, recuperación, rebuild, migración y límites de
reconciliación. Reitera que los fingerprints algebraicos no son
criptográficos.

## Dependencias y paquete

`validation/rc/dependency-inventory-v1.json` registra los cuatro lockfiles,
dependencias runtime directas, política de features y frontera del paquete.
`tools/audit_rc_package.sh` verifica que el paquete raíz no incluya datasets,
fuzz state, targets ni fixtures y ejecuta un archive dry-run offline de
`microfield`.

El paquete raíz queda en 100 ficheros y ya declara versión para su dependencia
`microfield`. Su archive completo se aplaza hasta disponer de un registry o
staging source para `microfield`; Cargo no permite preparar offline un paquete
cuya dependencia versionada aún no existe en el índice. Esto bloquea
publicación externa, no consumo interno por path/git. La licencia y el gate de
advisories/SBOM siguen clasificados explícitamente como blockers de publicación
externa.

Los defaults del paquete raíz dejan de activar `legacy`: ahora son
`signatures,graph`. Legacy continúa disponible mediante feature explícita y se
mantiene en CI.

## CI

El job requerido `RC.9 external consumer` ejecuta formato, tests, Clippy y el
binario end-to-end en x86-64 y AArch64, sube sus artefactos persistentes y
ejecuta el audit de paquete en x86-64.

Evidencia local:

```text
cargo test --manifest-path test-fixtures/rc-consumer/Cargo.toml --all-targets --locked
    2 passed
cargo clippy --manifest-path test-fixtures/rc-consumer/Cargo.toml --all-targets --locked -- -D warnings
    PASS
bash tools/audit_rc_package.sh
    root inventory 100 files; microfield package 1.3 MiB; PASS
```

El gate RC.9 se considerará integrado cuando ambas arquitecturas remotas estén
verdes. RC.10 ya está implementado localmente y consume estos artifacts; el
siguiente paso es publicar el commit y ejecutar el gate remoto conjunto.
