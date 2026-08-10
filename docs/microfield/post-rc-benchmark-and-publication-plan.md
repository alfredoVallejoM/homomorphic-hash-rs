# Plan de maduración: benchmarks, integración y publicación

Fecha: 9 de agosto de 2026.

Estado actual: `fe528c4` obtuvo `ReadyForInternalUse` en x86-64/AArch64 y
B.1–B.3 ya entregaron protocolo, harness, piloto y campaña profunda. La
ejecución profunda es `Informative`; la integración continúa aplazada hasta su
réplica `Controlled`, definida normativamente en
[`pre-rc-benchmark-protocol.md`](pre-rc-benchmark-protocol.md). Este plan no
cambia la semántica de las firmas homomórficas: son resúmenes algebraicos no
criptográficos.

## P.0 — integrar y fijar el checkpoint — aplazado hasta réplica controlada

- abrir una PR de `rc/rc7-correctness` a `main`;
- revisar el diff RC.7–RC.10, hacer merge sin perder historia y ejecutar CI
  post-merge;
- crear un tag anotado solo sobre el merge verde;
- conservar el artifact RC.10 y los informes RC.8/RC.9 del commit evaluado.

Gate: `main` y el tag apuntan a un commit verde y recuperable. PR, merge y tag
son escrituras remotas distintas y requieren autorización explícita.

## B.1–B.3 — benchmark profundo — cerrado informativo antes de P.0

RC.8 ya es un gate sólido de capacidad y regresión interna. B.1–B.3 han
construido y ejecutado una campaña distinta, destinada a presentar resultados
reproducibles. Conserva 66.500 observaciones profundas, 44/44 celdas precisas,
22 comparaciones y ocho curvas. El protocolo pre-RC prevalece sobre este
resumen:

- guardar cada observación cruda, no solo p50/p95/p99;
- ejecutar procesos independientes por celda, con mínimo 30, parada por
  precisión, warmup y orden aleatorizado reproducible;
- publicar mediana, p95/p99, MAD e intervalos bootstrap del 95 %;
- registrar CPU exacta, microcode, kernel, governor, turbo, frecuencia,
  memoria, toolchain, flags, backend detectado y temperatura cuando exista;
- fijar afinidad y usar runners dedicados para los resultados publicables;
- separar preparación, generación, aplicación, serialización, persistencia e
  I/O end-to-end;
- ampliar curvas de escala para campos, firmas, deltas, archivo/árbol, DB,
  reconciliación y grafos;
- comparar rutas directas, fachada comprobada, incremental y rebuild exacto;
- versionar JSON/CSV crudos, checksums, scripts de gráficas y un informe que
  pueda regenerarse desde un commit limpio.

No se comparará con otra biblioteca hasta congelar una operación equivalente,
el mismo nivel de seguridad —aquí ninguno criptográfico—, hardware, inputs y
tratamiento estadístico. Un gráfico no sustituirá los datos crudos.

Gate local satisfecho: una máquina limpia reproduce tablas desde raw. El
manifest histórico `publication-controlled-v1.json` sirve como seed y control
de regresión, pero ya no basta para cerrar claims. El gate vigente es la
campaña extensiva descrita en
[`c3-extensive-campaign-plan.md`](c3-extensive-campaign-plan.md), con cobertura
de toda la superficie y réplica Intel x86-64, AMD x86-64 y AArch64.

## B.4 — vertical de base de datos real

- definir un adapter público que mapee commit/LSN a revisión y namespace;
- implementar primero un fixture durable reproducible con WAL y después el
  adapter del motor objetivo;
- cubrir writers concurrentes, retry, crash/restart, truncación de log,
  migración de schema y rebuild desde tabla autoritativa;
- medir generación y aplicación de transacción, WAL, fsync, replay,
  reconciliación y fallback con I/O real;
- decidir si reconciliación v1 queda congelada para conjuntos o si una v2 debe
  representar multiplicidad.

Gate: la firma nunca autoriza una retirada; la transacción exacta y la base de
datos son la autoridad. El estado tras restart coincide con rebuild.

## P.1 — ingeniería de publicación externa

- elegir y añadir licencia; crear `SECURITY.md`, `CONTRIBUTING.md` y
  `CHANGELOG.md`;
- fijar semver, política de MSRV y compatibilidad/migración de wires;
- añadir advisories, SBOM y política de actualización de dependencias;
- preparar una fuente publicable o staged para `microfield` y verificar ambos
  archives desde un consumidor limpio;
- definir soporte, plataformas y claims públicos admitidos.

Gate: el package audit completo, documentación, examples y consumidor desde el
artefacto empaquetado pasan sin paths privados.

## Disciplina de cierre

Cada fase se considera terminada únicamente después de:

1. código, tests, artifacts y documentación actualizados;
2. validación local proporcional al riesgo;
3. commit específico con árbol limpio;
4. push inmediato de la rama;
5. CI remoto verde y enlace de evidencia.

No se acumularán fases terminadas sin publicar. PR, merge, tag y release se
mantienen como autorizaciones separadas.
