# Índice y ciclo de vida de la documentación

Fecha de inventario: 10 de agosto de 2026.

Este directorio conserva tanto contratos vigentes como planes e informes
históricos. La redacción en futuro de un plan ejecutado y la frase “siguiente
paso” de un informe de cierre describen su momento original; no reabren por sí
solas trabajo ya integrado.

## Orden de precedencia

Ante una contradicción se consulta, en este orden:

1. [`current-status-and-next.md`](current-status-and-next.md), para estado,
   riesgos y prioridades actuales;
2. [`supported-surface-v1.json`](../../validation/rc/supported-surface-v1.json),
   para clasificación machine-readable de capacidades;
3. schemas, manifests y artefactos versionados bajo `validation/` y
   `crates/microfield/`;
4. [`contracts.md`](contracts.md), [`architecture.md`](architecture.md) y ADR
   aceptados, para contratos y decisiones;
5. informes finales, para evidencia de un hito;
6. planes ejecutados, roadmaps antiguos y auditorías iniciales, como contexto
   histórico.

Los tests prevalecen sobre cifras narrativas de cobertura. Cambiar una
expectativa de producto exige actualizar también el inventario ejecutable y su
test en `tests/rc_supported_surface.rs`.

## Documentos vigentes

| Documento | Función |
|---|---|
| [`current-status-and-next.md`](current-status-and-next.md) | fotografía ejecutiva y próximo orden |
| [`release-candidate-readiness-plan.md`](release-candidate-readiness-plan.md) | gates RC.7–RC.10 |
| [`rc-8-capacity-report.md`](rc-8-capacity-report.md) | workloads, SLO, break-even y fallback RC.8 |
| [`rc-9-integration-report.md`](rc-9-integration-report.md) | consumidor, dependencias y package audit RC.9 |
| [`rc-9-operations-runbook.md`](rc-9-operations-runbook.md) | operación, recuperación y migración |
| [`rc-10-decision-report.md`](rc-10-decision-report.md) | contrato, evidencia y estados del dictamen RC.10 |
| [`pre-rc-benchmark-protocol.md`](pre-rc-benchmark-protocol.md) | metodología, matriz y gates B.1–B.3 previos a RC |
| [`pre-rc-b2-benchmark-harness-report.md`](pre-rc-b2-benchmark-harness-report.md) | workers, raw data, estadística y smoke B.2 |
| [`pre-rc-b3-benchmark-results.md`](pre-rc-b3-benchmark-results.md) | piloto, campaña profunda, curvas, cruces y límites B.3 |
| [`pre-rc-bulk-scaling-results.md`](pre-rc-bulk-scaling-results.md) | lotes coalescidos del árbol, DB por particiones y frontier de densidad |
| [`algesum-postgresql-results.md`](algesum-postgresql-results.md) | integración PostgreSQL, change stream y política adaptativa |
| [`pre-rc-comprehensive-campaign-plan.md`](pre-rc-comprehensive-campaign-plan.md) | matriz C0–C3 de todas las estructuras, priorizando firmas y grafos |
| [`c3-extensive-campaign-plan.md`](c3-extensive-campaign-plan.md) | especificación C3 extensiva: 15 suites, tres carriles, réplica multihost y sistemas reales |
| [`c3-p0-implementation-and-preflight-report.md`](c3-p0-implementation-and-preflight-report.md) | expansor factorial, inventario de gaps y primer preflight F1/F2 |
| [`c3-f3-s3-implementation-and-preflight-report.md`](c3-f3-s3-implementation-and-preflight-report.md) | batería F3–S3: batch, runtime, herramientas, firmas, estado, deltas y journals |
| [`c3-t1-r1-d1-implementation-and-preflight-report.md`](c3-t1-r1-d1-implementation-and-preflight-report.md) | archivos/árboles, reconciliación y base de datos en memoria |
| [`pre-rc-comprehensive-smoke-results.md`](pre-rc-comprehensive-smoke-results.md) | C1 de 65 celdas y piloto PostgreSQL hasta un millón de filas |
| [`pre-rc-comprehensive-pilot-results.md`](pre-rc-comprehensive-pilot-results.md) | C2 de firmas K/fragmentación, grafos y PostgreSQL repetido a un millón |
| [`post-rc-benchmark-and-publication-plan.md`](post-rc-benchmark-and-publication-plan.md) | integración, benchmark publicable, DB real y release externa |
| [`phase-6-g15-internal-readiness-plan.md`](phase-6-g15-internal-readiness-plan.md) | desglose histórico G15 y correspondencia con RC |
| [`contracts.md`](contracts.md) | contratos algebraicos, batch, generación y firmas |
| [`architecture.md`](architecture.md) | capas y dependencias actuales |
| [`design-principles.md`](design-principles.md) | criterios de diseño |
| [`pattern-catalog.md`](pattern-catalog.md) | patrones implementados |
| [`runtime-codegen-compatibility.md`](runtime-codegen-compatibility.md) | matriz ABI congelada |
| [`unsafe-audit.md`](unsafe-audit.md) | frontera `unsafe` vigente |
| [`binary-field-factory.md`](binary-field-factory.md) | guía de consumo del generador binario |

Los ADR `0001`–`0034` están aceptados. Una decisión posterior puede ampliar o
reabrir otra anterior; en particular, ADR 0031 reabre la autoridad exacta de
G6–G7 y ADR 0032–0034 fijan assurance, comparación pareada y pipeline/delta.

## Planes activos o parcialmente satisfechos

- `release-candidate-readiness-plan.md`: RC.0–RC.10 implementados; RC.7–RC.10
  tienen gate remoto verde en la rama RC y están pendientes de integración en
  `main`.
- `pre-rc-benchmark-protocol.md`: B.1–B.3 cerrados con evidencia informativa;
  la extensión bulk del árbol/DB también está ejecutada como `Informative`; la
  réplica `Controlled` de ambas es precondición para promover o integrar la RC.
- `pre-rc-comprehensive-campaign-plan.md`: C1 y C2 ejecutados; C2 conserva 47
  celdas de firmas/grafos y 33 muestras PostgreSQL verificadas. La selección
  DB densa y la telemetría exacta ya se corrigieron. C3 se ha replanificado
  como campaña extensiva de 12.000+ casos semánticos, 2.500–4.000 celdas de
  timing y 150–300 escenarios. P0 y F1–S3 ya aportan 2.610 celdas generadas y
  sus preflights; quedan estructuras, DB, grafos, semántica/sistemas y réplica
  multihost `Controlled`.
- `post-rc-benchmark-and-publication-plan.md`: primera integración PostgreSQL
  ejecutada; logical decoding, concurrencia, corpus real y release externa
  continúan después de reevaluar B.1–B.3.
- `phase-6-g15-internal-readiness-plan.md`: G15.0–G15.4 materializados por
  RC.0–RC.6; G15.5 parcial y G15.6–G15.9 abiertos.
- `phase-6-validation-plan.md`: harness V1–V6 ejecutado; continúan campañas de
  evidencia y baselines, no una reimplementación del laboratorio.
- `relational-green-invariant-research.md`: propuesta de investigación sin
  compromiso de producto ni claim de novedad.

## Planes e informes históricos

Los siguientes grupos están cerrados o sustituidos y se conservan para
trazabilidad:

- `phase-1-*` a `phase-5-*`;
- `phase-6-pre-canon-*`, `phase-6-fast-graph.md` y
  `phase-6-g3-final-report.md` a `phase-6-g7-final-report.md`;
- `phase-6-g8-g9-implementation-report.md` y los cierres G10–G14;
- `phase-6-final-report.md`, que solo cierra el baseline G0–G7;
- `audit.md`, auditoría inicial, y `phases-3-7-roadmap.md`;
- `github-integration-and-remote-validation-plan.md`, ejecutado mediante la PR
  #1 y el tag `internal-rc6-integrated`;
- informes RC.0–RC.6, ya integrados en `main`.

Los hallazgos históricos no deben borrarse: documentan por qué existen las
fronteras actuales. Sí deben llevar una nota de ciclo de vida cuando una frase
pueda confundirse con el presente.

## Documentos raíz y de crates

- [`../../README.md`](../../README.md): entrada de usuario y comandos
  recomendados.
- [`../../planificacion.md`](../../planificacion.md): especificación histórica
  F0–F2 con apéndices posteriores; no es el backlog vigente.
- [`../../crates/microfield/README.md`](../../crates/microfield/README.md): API
  y ejemplos del núcleo de campos.
- [`../../crates/validation-lab/README.md`](../../crates/validation-lab/README.md)
  y [`../../validation/f6/README.md`](../../validation/f6/README.md): ejecución
  y artefactos del laboratorio privado.
- [`../../crates/algesum-postgres-lab/README.md`](../../crates/algesum-postgres-lab/README.md):
  campaña PostgreSQL reproducible y sus límites.

## Regla de mantenimiento

Cada cambio funcional debe actualizar, en el mismo commit cuando corresponda:

1. código, tests y schema/wire;
2. inventario de superficie soportada;
3. contrato o ADR afectado;
4. fotografía actual y siguiente plan;
5. informe de evidencia si cierra un gate.

No se actualizarán cifras históricas para que parezcan actuales. Las nuevas
mediciones se añaden con commit, toolchain, hardware, comando y fecha.
