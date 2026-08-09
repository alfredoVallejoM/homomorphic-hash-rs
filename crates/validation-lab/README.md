# Microfield validation lab

Este crate privado y no publicable ejecuta las campañas reproducibles F6.V.
No forma parte de la API de producto: separa los oráculos, corpus, baselines y
resultados científicos del camino crítico de la librería.

Estado, 9 de agosto de 2026: los artefactos semánticos se regeneran con diff
vacío y el workflow principal los reproduce en x86-64 y AArch64. RC.8 añade
una campaña host-specific con SLO, percentiles, throughput, asignaciones,
memoria temporal pico, bytes persistidos/comunicados, I/O evitado y curvas de
punto de equilibrio. Los resultados de tiempo nunca se versionan como golden
entre máquinas.

El harness pre-RC publicable es una campaña separada: lanza procesos aislados,
guarda observaciones crudas y aplica bootstrap jerárquico. El perfil `smoke`
comprueba el mecanismo, pero nunca habilita claims.

```bash
cargo run -p microfield-validation-lab -- semantic \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/semantic-v1.json

cargo run --release -p microfield-validation-lab -- performance \
  --manifest validation/f6/manifest.json \
  --out /tmp/f6-performance.json

cargo run --release -p microfield-validation-lab -- g11 \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/g11-v1.json

cargo run --release -p microfield-validation-lab -- g12 \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/g12-v1.json

cargo run --release -p microfield-validation-lab -- g13-g14 \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/g13-g14-v1.json

cargo run --release -p microfield-validation-lab -- rc8-capacity \
  --manifest validation/rc/capacity-manifest-v1.json \
  --out /tmp/rc8-capacity.json

cargo run --release -p microfield-validation-lab -- rc8-compare \
  --manifest validation/rc/capacity-manifest-v1.json \
  --baseline /tmp/rc8-baseline.json \
  --candidate /tmp/rc8-candidate.json \
  --out /tmp/rc8-regression.json

cargo run -p microfield-validation-lab -- rc10-decision \
  --manifest validation/rc/decision-manifest-v1.json \
  --capacity-report /tmp/rc8-x86_64.json \
  --capacity-report /tmp/rc8-aarch64.json \
  --consumer-report /tmp/rc9-x86_64/rc9-report.json \
  --consumer-report /tmp/rc9-aarch64/rc9-report.json \
  --required-ci-gates-passed \
  --out /tmp/rc10-decision.json

cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/smoke-v1.json \
  --run-dir /tmp/microfield-publication-smoke

cargo run --release -p microfield-validation-lab --locked -- \
  publication-analyse \
  --manifest validation/benchmarks/manifests/smoke-v1.json \
  --run-dir /tmp/microfield-publication-smoke
```

`semantic`, `g11`, `g12` y `g13-g14` son deterministas. `g11` fija un split
discovery/holdout y compara los canales de loops/Green sobre el corpus n=8
autenticado.
`g12` compara el matcher pareado contra formas canónicas, relabelings grandes y
los pares adversariales CFI/SRG.
`g13-g14` congela el enrutado por niveles, los positivos exactos verificados y
la equivalencia diferencial de las rutas incremental/fallback.
`performance` captura hardware y tiempos y nunca se usa como golden test entre
máquinas.

`publication-campaign` rechaza directorios no vacíos, baraja celdas con semilla
versionada y ejecuta cada par celda/réplica en un proceso nuevo.
`publication-analyse` regenera JSON, CSV, comparaciones, informe y checksums
desde `raw/workers.jsonl`. `Smoke` y `Informative` mantienen
`claims_allowed=false`.

`rc8-capacity` ejecuta 37 rutas congeladas sobre campos, todas las familias de
firmas estáticas, deltas, archivos/árbol, base de datos, reconciliación y
grafos. El manifest liga cada ruta a su SLO y a una acción explícita cuando una
ruta relativa deja de ser rentable. `rc8-compare` solo acepta informes con el
mismo campaign, hardware, toolchain, features observables, escalas y checksums;
rechaza una regresión p50/p95 o de memoria superior al 3 %, incluida cualquier
nueva asignación en una ruta previamente zero-allocation.

`rc10-decision` ensambla evidencia RC.7–RC.9, commit y estado del árbol,
toolchains, hardware, clasificación de capacidades, hashes de corpus y
limitaciones declaradas. Su salida es exactamente `ReadyForInternalUse`,
`Conditional` o `NotReady`: un fallo produce `NotReady`; evidencia ausente o
un árbol sin commit limpio produce `Conditional`. El workflow es la autoridad
para `ReadyForInternalUse`, porque aporta x86-64, AArch64 y todos los gates
requeridos desde el mismo commit limpio.
