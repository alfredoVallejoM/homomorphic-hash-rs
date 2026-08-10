# C3 F3–S3: implementación y preflight

Fecha: 2026-08-10. Estado: **implementado y ejecutado como Smoke;
`Controlled` pendiente**.

Este hito amplía C3 desde los campos estáticos F1/F2 hasta batch, packed,
algoritmos derivados, campos dinámicos, herramientas y todas las familias de
firmas previstas en S1–S3. Las firmas y hashes homomórficos de Algesum son
**resúmenes algebraicos no criptográficos**: estos resultados no implican
autenticación, resistencia criptográfica a colisiones ni seguridad adversaria.

## Alcance materializado

El plan factorial versionado
`validation/benchmarks/c3-f3-s3-factor-plan-v1.json` genera siete shards:

| Shard | Celdas publicables | Preflight | Cobertura principal |
|---|---:|---:|---|
| F3 batch/packed | 54 | 3 | batch portable, packed owned y packed view |
| F3 Horner | 120 | 2 | un polinomio/muchos puntos y muchos polinomios/un punto |
| F3 derivados | 125 | 7 | scans, batch inversion, powers y masks |
| F4 runtime/tools | 36 | 6 | ciclo de vida dinámico, manifests y consumidor externo limpio |
| S1 firmas base | 784 | 14 | cuatro leyes, remove, wire/restore y patrones de entrada |
| S2 multievaluación | 378 | 9 | K=2/3/4/8/16, runtime, fragmentación y wire/restore |
| S3 estado/deltas | 615 | 15 | tracked, snapshots, deltas, journals y rechazo de corrupción |
| **Total F3–S3** | **2.112** | **56** | **26 operaciones nuevas** |

Sumadas a las 498 celdas F1/F2, hay **2.610 celdas C3-Scaling definidas**.
Esto satisface el intervalo normativo de 2.500–4.000 sin rebajar el objetivo de
12.000 casos C3-Semantic ni el de 150–300 escenarios C3-Systems, que son
carriles distintos y todavía no están cerrados.

El inventario machine-readable marca F3, F4, S1, S2 y S3 como implementadas y
sin operaciones ocultas pendientes. La regeneración de todos los manifests es
determinista y está bloqueada por tests byte a byte.

## Correcciones efectuadas durante el preflight

1. El muestreo del expansor ahora distingue variantes que comparten operación
   pero cambian de estrategia estática; antes podía seleccionar una variante
   diferente a la declarada por la celda.
2. S1 y S3 se ampliaron para representar todas las leyes de firma, snapshots
   compactos y tracked, deltas de append/trim y caminos corruptos/truncados.
3. La prueba de consumidor externo elimina únicamente su target temporal
   aislado por PID antes de cada acción y ejecuta `cargo check --locked
   --offline`; por tanto mide una compilación limpia, no incremental.
4. Dos tests de regresión ejecutan dos veces cada acción preflight y comparan
   checksums; otro contrasta scalar, packed-owned y packed-view.

## Resultado observado

Se ejecutaron siete runs independientes bajo
`validation/benchmarks/runs/c3-*-preflight-*`, consolidados en
`c3-f3-s3-preflight-summary-v1.json`:

| Métrica | Resultado |
|---|---:|
| Celdas | 56 |
| Procesos worker | 112 |
| Observaciones | 560 |
| Celdas dentro del CI configurado de Smoke | 56/56 |
| Checksums inestables | 0 |
| Celdas sin asignación en la región medida | 17 |
| Celdas con asignación | 39 |
| Bytes acumulados en probes de asignación | 11.827.084 |
| Pico máximo de una acción | 115.333 bytes |
| Bytes de artefactos | 325.896 |

Las doce variantes F3 fueron allocation-free dentro de la región medida. Las
asignaciones F4/S1–S3 corresponden a acciones end-to-end que construyen campos,
serializan, restauran o retienen estado; no se clasifican automáticamente como
defectos. La compilación externa crea procesos hijos, cuyas asignaciones no son
visibles para el contador del proceso padre.

La compilación limpia del consumidor fue la celda dominante, con mediana
aproximada de 9,95 s. Horner muchos-polinomios fue la operación puramente
algebraica más lenta del preflight, alrededor de 49 ms en la muestra elegida.
Estas cifras sirven para presupuestar la campaña: no son claims de rendimiento.

## Qué puede y qué no puede concluirse

El preflight demuestra que todas las acciones seleccionadas se construyen,
terminan, emiten checksums repetibles y pueden ser analizadas por el harness.
También confirma que el volumen factorial está dentro del presupuesto. No
demuestra todavía portabilidad multihost, rendimiento publicable, colas
sostenidas, recuperación real ni superioridad frente a alternativas.

La clasificación es `Smoke`, con `claims_allowed=false`: se ejecutó desde un
árbol con cambios sin confirmar, en un host compartido y sin afinidad/frecuencia
controladas. El umbral de precisión del 25 % solo detecta fallos gruesos del
harness. Ningún tiempo de este informe debe presentarse como benchmark final.

## Siguiente tramo C3

El orden de ejecución restante es:

1. T1, R1 y D1: árboles, reconciliación y estructuras de base de datos;
2. G1, G2, X1 y X2: grafos, exactitud acotada, corpora externos y comparación;
3. D2: PostgreSQL, persistencia, concurrencia, crash/restart y replay;
4. cerrar C3-Semantic y C3-Systems;
5. ejecutar C3-Scaling `Controlled` en Intel x86-64, AMD x86-64 y AArch64,
   con al menos 30 procesos por celda.

Con 2.112 celdas F3–S3, solo esta parte exige al menos 63.360 procesos en la
campaña controlada. Debe fragmentarse por shard y host conservando commit,
toolchain, manifiesto, entorno y evidencia cruda.
