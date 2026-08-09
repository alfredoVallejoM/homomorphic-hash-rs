# B.3 — escalabilidad y campaña profunda pre-RC

Fecha: 9 de agosto de 2026.

Estado: **cerrada con evidencia informativa; pendiente réplica controlada**.

Las firmas medidas son resúmenes algebraicos homomórficos **no
criptográficos**. Ningún resultado de este informe implica autenticación,
resistencia adversarial a colisiones o seguridad criptográfica.

## Resultado ejecutivo

B.3 ejecutó la misma matriz de 44 celdas en dos presupuestos. El piloto reunió
6.600 observaciones en 440 procesos; 42 celdas alcanzaron su objetivo de
precisión y dos quedaron explícitamente `Inconclusive`. La campaña profunda
reunió 66.500 observaciones en 1.330 procesos y alcanzó precisión en 44/44
celdas. Cuarenta y tres se detuvieron en el mínimo de 30 procesos y una amplió
automáticamente a 40.

La matriz contiene 22 comparaciones pareadas y ocho curvas con cinco o seis
escalas. Inputs, semilla, orden de ejecución, entorno, observaciones crudas,
agregados, CSV, informe generado y checksums están versionados. Regenerar los
agregados desde `raw/workers.jsonl` produjo exactamente el mismo digest en
ambas campañas.

Las dos ejecuciones se clasificaron `Informative`, no `Controlled`: se usó un
binario release desde un árbol limpio, pero el host interactivo no fue
declarado dedicado ni se atestiguó afinidad exclusiva. Por ello
`claims_allowed=false` aunque todas las celdas profundas sean precisas.

## Evidencia versionada

| Campaña | Código medido | Presupuesto | Resultado | Digest de `checksums.txt` |
|---|---|---|---|---|
| `pre-rc-b3-pilot-scaling-v1` | `42874c794c67c040ed3e6b7af90c72b7e1a36fe9` | 10 procesos × 15 observaciones | 42 `Precise`, 2 `Inconclusive` | `6ae981ddf09f17d3435a4cebdcf331dd70cbc6ad5bac17c1697a120514a6ade6` |
| `pre-rc-b3-publication-informative-v1` | `49828e390502283cc5e7f80faf683838de21b7bf` | 30–50 procesos × 50 observaciones | 44 `Precise` | `5dd21e2cccec83e494fc1b2fa603c61b114d27c98a4b11c4d8868e0cd55c19f6` |

Los artefactos completos están en:

- [`pre-rc-b3-pilot-scaling-v1`](../../validation/benchmarks/runs/pre-rc-b3-pilot-scaling-v1/report.md);
- [`pre-rc-b3-publication-informative-v1`](../../validation/benchmarks/runs/pre-rc-b3-publication-informative-v1/report.md).

El commit `49828e3` solo añade la evidencia del piloto; no altera las
operaciones medidas. Ambos entornos registran `tree_clean=true` y arquitectura
`x86_64`.

## Matriz ejecutada

| Área | Variantes pareadas | Escalas |
|---|---|---|
| GF(2^128) | batch con engine detectado / bucle escalar directo | 1, 8, 64, 512, 4.096, 65.536 elementos |
| firma aditiva | merge homomórfico / reconstrucción desde items | 8, 64, 512, 4.096, 65.536 items |
| summary tree de 256 KiB | edición local / reconstrucción completa | 1, 64, 4.096, 16.384, 65.536, 262.144 bytes editados |
| DB en memoria de 512 filas | transacción end-to-end / reconstrucción de tabla | 1, 8, 32, 128, 512 mutaciones |

Cada ratio es `variante / baseline`: menos de 1 favorece la variante y más de
1 favorece el rebuild. Los pares comparten semilla y réplica; los intervalos
son bootstrap jerárquicos del 95 %.

## Hallazgos de la campaña profunda

| Pregunta | Resultado observado | Lectura válida |
|---|---|---|
| Engine detectado frente a escalar | ratio 0,1216 en 1 elemento y 0,0359 en 65.536 | entre 8,22× y 27,85× menos tiempo en este host y operación |
| Merge aditivo frente a rebuild | ratio 0,00285 en 8 items y 0,000000462 en 65.536 | merge de firmas ya construidas casi constante; no incluye construir las entradas |
| Edición local del árbol | ratio 0,01734 en 1 byte, 0,2522 en 64 KiB y 1,0037 en 256 KiB | ventaja clara hasta 64 KiB; cruce observado entre 64 y 256 KiB |
| Transacción DB incremental | ratio 0,04231 en 1 mutación, 0,3106 en 8 y 1,1172 en 32 | cruce observado entre 8 y 32 mutaciones sobre 512 filas |

Los intervalos profundos no cruzan 1 en los puntos que delimitan ambos
cruces: árbol a 64 KiB `[0,25216; 0,25228]` y 256 KiB
`[1,00340; 1,00399]`; DB a 8 mutaciones `[0,30984; 0,31105]` y 32
`[1,11637; 1,11840]`.

Las pendientes log-log estimadas son coherentes con la estructura del trabajo:

- campo: 0,993 escalar y 0,909 engine detectado;
- firma aditiva: 0,975 rebuild y 0,002 merge;
- summary tree de tamaño fijo: 0,000 rebuild y 0,270 incremental dentro del
  rango mixto medido;
- DB de 512 filas: 0,004 rebuild y 0,958 incremental respecto a mutaciones.

Son descripciones del rango y host medidos, no pruebas asintóticas.

## Qué demuestra y qué no

La evidencia permite tomar decisiones internas defendibles: mantener el batch
detectado, conservar el merge algebraico, activar rebuild del árbol cuando la
edición se aproxima al tamaño total y usar un fallback DB antes de 32
mutaciones para esta tabla/carga. Los puntos de cruce deben modelarse por
tamaño relativo en la política final, no copiarse como constantes universales.

Todavía no permite:

- publicar cifras principales o compararlas entre máquinas;
- extrapolar a otras CPUs, AArch64, otros tamaños de fila o distribuciones;
- atribuir propiedades criptográficas a las firmas;
- caracterizar WAL, `fsync`, contención, crash/restart o una base de datos
  externa;
- afirmar complejidad asintótica a partir de cinco o seis puntos.

La matriz B.3 profundiza cuatro hipótesis centrales y el smoke B.2 conserva
cobertura funcional de todas las familias. No pretende que cada una de las 21
familias del smoke tenga ya cinco escalas; esa ampliación queda priorizada por
claims y consumidor durante B.4 y la preparación de publicación.

## Reproducción

Validación de integridad:

```text
cd validation/benchmarks/runs/pre-rc-b3-publication-informative-v1
sha256sum -c checksums.txt
```

Regeneración determinista desde raw:

```text
cargo run --release -p microfield-validation-lab --locked -- \
  publication-analyse \
  --manifest validation/benchmarks/manifests/publication-informative-v1.json \
  --run-dir validation/benchmarks/runs/pre-rc-b3-publication-informative-v1
```

Una nueva ejecución debe usar otro `run-dir`: el launcher se niega a
sobrescribir evidencia existente.

## Decisión y siguiente gate

B.1, B.2 y B.3 están cerradas en código, datos y documentación. La decisión es:

- `GO` para usar los resultados como guía de optimización y fallback interno;
- `NO-GO` para claims externos y promoción inmediata de RC;
- siguiente acción: ejecutar `publication-controlled-v1.json` en un host
  realmente dedicado, con afinidad fijada y atestiguaciones verificables,
  primero en x86-64 y después en AArch64;
- después: reevaluar la RC y abordar B.4, que añade DB real, WAL, `fsync`,
  concurrencia y recuperación.
