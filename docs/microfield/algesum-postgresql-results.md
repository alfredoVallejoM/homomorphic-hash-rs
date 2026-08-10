# Integración y escalado PostgreSQL de Algesum

Fecha: 2026-08-10. Clasificación: **Informative**; no habilita afirmaciones
públicas de rendimiento.

## Resultado

Algesum se ha probado contra PostgreSQL 17 sobre 65.536 filas persistidas, 256
particiones y lotes de 256 a 65.536 modificaciones. Cada una de las 18
transacciones de la campaña final terminó con:

- filas PostgreSQL idénticas al estado exacto esperado;
- resumen incremental/adaptativo idéntico a una reconstrucción completa;
- revisión Algesum y posición de commit correctamente separadas;
- aplicación atómica y sin pérdida de filas.

Medianas observadas, en microsegundos:

| Modificaciones | Densidad | PostgreSQL | Algesum | Ruta | Rebuild de referencia |
|---:|---:|---:|---:|---|---:|
| 256 | 0,39 % | 8.122 | 14.293 | incremental | 158.752 |
| 4.096 | 6,25 % | 45.666 | 39.087 | incremental | 158.497 |
| 16.384 | 25 % | 136.555 | 105.581 | incremental | 159.160 |
| 32.768 | 50 % | 283.266 | 196.418 | incremental | 158.056 |
| 49.152 | 75 % | 447.895 | 262.421 | híbrida por partición | 158.361 |
| 65.536 | 100 % | 587.201 | 297.852 | rebuild por partición | 160.527 |

La ruta adaptativa `2/3` conserva el camino incremental hasta el 50 % y reduce
la mediana de aplicación respecto de la campaña incremental pura de 281.746 a
262.421 µs al 75 % (6,9 %) y de 376.052 a 297.852 µs al 100 % (20,8 %).

El rebuild de referencia no es una operación equivalente: construye un objeto
nuevo desde filas ya disponibles y no valida before-images ni publica una
transacción sobre el estado existente. Se conserva para localizar el coste
algorítmico mínimo, no como competidor transaccional directo.

## Correcciones incorporadas

- `DatabaseChangeStreamAdapter` separa posiciones externas dispersas de las
  revisiones contiguas de Algesum.
- Redelivery exacta de la última transacción es idempotente.
- Posiciones regresivas, fuentes incorrectas, checkpoints divergentes y
  reutilización conflictiva fallan de forma cerrada.
- La reconstrucción autoritativa valida filas modificadas, filas no modificadas,
  cardinalidad y unicidad sin construir mapas completos redundantes.
- El laboratorio PostgreSQL hace persistentes los resultados crudos y permite
  recalibrar los dos umbrales de política.

## Evidencia y límites

- incremental puro: [`pre-rc-postgresql-incremental-v1`](../../validation/benchmarks/runs/pre-rc-postgresql-incremental-v1/report.json);
- adaptativa inicial `1/2`: [`pre-rc-postgresql-adaptive-v1`](../../validation/benchmarks/runs/pre-rc-postgresql-adaptive-v1/report.json);
- adaptativa seleccionada `2/3`: [`pre-rc-postgresql-adaptive-v2`](../../validation/benchmarks/runs/pre-rc-postgresql-adaptive-v2/report.json).

La campaña usa un único cliente y una tabla sintética reproducible. Todavía no
mide logical decoding/WAL real, crashes, 16–256 clientes concurrentes ni el
corpus NYC TLC. Esos puntos permanecen como siguiente fase antes de la RC.
