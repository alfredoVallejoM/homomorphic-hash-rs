# RC.8 — informe de capacidad y fallback medido

Fecha: 9 de agosto de 2026.

Estado: implementación y validación local completas; evidencia remota x86-64
y AArch64 pendiente de ejecutar en GitHub antes de declarar RC.8 integrado.

## Alcance ejecutable

`validation/rc/capacity-manifest-v1.json` congela 37 workloads y sus SLO. La
campaña cubre:

- scalar, batch detectado, kernel packed y pipeline pack/mul/unpack;
- ingestión de las seis familias estáticas de firmas, merge, concatenación,
  tracking y restauración;
- generación, aplicación, rollback algebraico y replay de deltas;
- edición local, append, cambio de fronteras, fallback gobernado y rebuild de
  archivos/árboles;
- generación/aplicación/fallback de transacciones, rebuild, log/replay y
  reconciliación de base de datos;
- filtro preparado, canonización exacta y reutilización del DAG de grafos.

Las firmas medidas son fingerprints algebraicos homomórficos **no
criptográficos**. Ningún resultado de rendimiento modifica esa clasificación.

Cada muestra publica p50/p95/p99, throughput, número y bytes totales de
asignaciones, memoria temporal pico, bytes wire, bytes persistentes, I/O
evitado y checksum semántico. La generación y la aplicación se miden por
separado donde existe un artefacto delta/transacción.

## Punto de equilibrio local observado

La ejecución local de referencia produjo 37/37 SLO verdes. En un archivo de
262.144 bytes, la recomposición local fue rentable hasta 65.536 bytes editados;
la edición de los 262.144 bytes cruzó a ratio 1,003 frente a rebuild. En una
tabla de 512 filas, 32 mutaciones tuvieron ratio 0,951 y 128 mutaciones ratio
3,61 frente a rebuild.

Por ello el manifest fija, de forma conservadora y revisable por hardware:

- `file_max_incremental_edit_bytes = 65536`;
- `database_max_incremental_mutations = 32`.

Estos límites son ejecutables mediante `SummaryEditPolicy` y
`DatabaseApplyPolicy`. El fallback de base de datos exige filas objetivo
autoritativas, reconstruye el estado exacto y verifica que coincida con todas
las before/after images antes de publicar. Una discrepancia deja estado y
revisión intactos.

Los valores anteriores son evidencia de este host, no promesas universales.
CI repite la campaña en x86-64 y AArch64 y conserva los JSON como artifacts.

## Gate de regresión

`rc8-compare` compara candidato y baseline en el mismo runner. Rechaza cambios
de campaign, entorno, workload, escala, iteraciones o checksum semántico. Para
cada ruta congelada rechaza regresiones superiores al 3 % en p50, p95, bytes
asignados o memoria pico; una asignación nueva desde baseline cero también
falla.

El workflow `Microfield` ejecuta `RC.8 capacity and regression` en ambas
arquitecturas y lo incluye en `Required gates`. La primera integración crea el
baseline; las PR posteriores lo comparan contra el commit objetivo en el mismo
runner.

## Evidencia local

```text
cargo test -p microfield-validation-lab --all-targets --locked        PASS
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
                                                                    PASS
cargo run --release -p microfield-validation-lab -- rc8-capacity …  37/37 PASS
cargo run --release -p microfield-validation-lab -- rc8-compare …   37/37 PASS
```

RC.8 se declarará integrado únicamente después de que ambas matrices remotas
publiquen artifacts verdes. RC.9 y RC.10 ya están implementados localmente; el
siguiente paso es publicar el commit y ejecutar el gate remoto conjunto.
