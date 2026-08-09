# B.2 — harness de benchmarks publicables

Fecha: 9 de agosto de 2026.

Estado local: implementación y smoke completos. El gate remoto x86-64/AArch64
se ejecuta con el commit de esta fase. Ninguna cifra smoke se considera un
resultado publicable.

Las firmas medidas son resúmenes algebraicos homomórficos **no
criptográficos**. El harness no introduce ni mide propiedades de seguridad.

## Resultado

B.2 entrega un laboratorio distinto de RC.8. La unidad experimental primaria
es un proceso nuevo y cada observación conserva proceso, semilla, orden,
duración, batch calibrado, unidades lógicas, asignaciones y checksum.

```text
manifest versionado
  -> launcher: orden determinista barajado y parada por precisión
     -> proceso worker aislado por celda/réplica
        -> JSON crudo autenticable
           -> análisis determinista
              -> JSON + CSV + comparaciones + Markdown + checksums
```

El comando `publication-analyse` reconstruye los agregados únicamente desde el
manifest, el entorno y `raw/workers.jsonl`. La regeneración no vuelve a medir y
debe producir los mismos bytes y checksums.

## Inventario smoke

`validation/benchmarks/manifests/smoke-v1.json` contiene 21 celdas y recorre
todas las familias mantenidas relevantes para la propuesta:

- GF(2^128), los dos GF(2^256), incluido `gf2_256_hh_v1`, Fp251,
  Goldilocks y batch detectado;
- firmas aditiva, secuencial, bidireccional, multiset y ambas
  multievaluaciones K=2;
- delta revisionado, summary tree, base de datos en memoria y reconciliación;
- filtro preparado, canonización exacta y reutilización de DAG;
- parseo y generación completa de un campo mediante el factory.

Las celdas representan las familias; B.3 amplía escalas y variantes
directo/incremental/rebuild. RC.8 conserva sus 37 rutas como gate operativo.

## Metodología implementada

- perfiles tipados `smoke`, `pilot` y `publication`;
- validación estricta de IDs, operaciones, escalas, baselines, curvas y límites;
- mínimo de 30 procesos exigido por código al perfil `publication`;
- calibración por duplicación hasta el tiempo objetivo sin restar un overhead
  supuesto del reloj;
- `black_box` sobre acción y resultado, más checksum dependiente de toda la
  ejecución;
- mediana de medianas por proceso como estimador primario;
- p95/p99 y sus intervalos mediante bootstrap jerárquico;
- MAD, intervalo bootstrap del 95 % y semianchura relativa;
- comparaciones pareadas por el mismo índice de proceso;
- slope log-log solo con al menos cinco puntos de escala;
- resultados fuera de precisión clasificados como `Inconclusive`, sin borrar
  outliers;
- clasificación `Smoke`, `Informative` o `Controlled`; esta última exige host
  dedicado y afinidad fija atestados explícitamente además de metadatos
  observables.

## Metadatos y artefactos

El environment report captura commit/limpieza, digest del binario, profile,
Rust/Cargo, arquitectura, OS/kernel, CPU/microcode/flags/cache, memoria,
governor/frecuencias/turbo, afinidad, contenedor y fecha UTC cuando están
disponibles. Los desconocidos se publican, no se rellenan por inferencia.

Cada run nuevo contiene:

```text
manifest.json              environment.json
execution-order.json       raw/workers.jsonl
aggregate.json             aggregate.csv
comparisons.csv            report.md
checksums.txt
```

El launcher rechaza sobrescribir un directorio no vacío. Los JSON temporales
de cada worker se consolidan en el JSONL y se eliminan después de verificar su
parseo, evitando duplicar los datos crudos.

## Evidencia local

Comandos ejecutados:

```text
cargo test -p microfield-validation-lab --all-targets --locked
cargo clippy -p microfield-validation-lab --all-targets --locked -- -D warnings
cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/smoke-v1.json \
  --run-dir /tmp/microfield-b2-final-smoke.PdH3bU
cargo run --release -p microfield-validation-lab --locked -- \
  publication-analyse \
  --manifest validation/benchmarks/manifests/smoke-v1.json \
  --run-dir /tmp/microfield-b2-final-smoke.PdH3bU
```

Resultado:

- 24 tests del laboratorio verdes;
- Clippy sin warnings;
- 21 celdas, 42 procesos aislados y 210 observaciones crudas;
- 21/21 celdas dentro de la tolerancia amplia del smoke;
- dos comparaciones pareadas ejercitadas;
- regeneración byte a byte con los mismos ocho checksums;
- clasificación `Smoke`, `claims_allowed = false`.

La precisión observada en dos procesos solo demuestra que funciona el gate;
no autoriza presentar sus tiempos.

## CI y frontera B.3

El job `Pre-RC publication harness smoke` repite la campaña en runners
x86-64 y AArch64, exige las 21 celdas, 42 workers y regeneración idéntica, y
publica el run como artifact no publicable. También forma parte de
`Required gates`.

B.3 debe añadir y ejecutar manifests `pilot` y `publication`, al menos cinco
escalas por curva y comparaciones pareadas directo/incremental/rebuild. Un host
interactivo producirá evidencia `Informative`; solo un host dedicado,
controlado y atestado podrá producir `Controlled` y habilitar claims.
