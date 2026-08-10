# C3-P0: implementación y primer preflight

Fecha: 2026-08-10. Estado: **infraestructura P0 implementada; primeras suites
F1/F2 probadas como Smoke**. Ninguna cifra de este informe permite claims.

Algesum trabaja con resúmenes y firmas algebraicas homomórficas **no
criptográficas**. El preflight no evalúa propiedades de seguridad.

## Resultado

C3-P0 convierte la planificación extensiva en artefactos ejecutables:

- ledger de las 30 capacidades y las 15 suites;
- inventario honesto de workloads implementados y ausentes por suite;
- plan factorial declarativo y schema;
- expansor determinista con cruce completo de factores primarios, cobertura
  pairwise de secundarios, baselines emparejados y shards;
- manifests separados `publication`, `smoke` y `preflight`;
- regeneración byte a byte comprobada por tests;
- 30 operaciones nuevas para los seis campos estáticos mantenidos;
- dos preflights aislados con raw, entorno, agregados y checksums.

## Operaciones añadidas

Para GF(2^128), las dos identidades GF(2^256), Fp251, Goldilocks y Fp256
genérico se registraron:

- suma total;
- producto total;
- cuadrado total;
- inversión total;
- encode/decode canónico total.

Cada acción lee inputs inmutables, reutiliza el buffer de salida y mezcla todos
los elementos en el checksum. Esto evita que warmup/calibración cambien el
estado algebraico y evita que el compilador conserve únicamente el último
elemento. Las 30 rutas midieron cero asignaciones dentro de la acción.

## Expansión factorial F1/F2

Los factores de escala son:

`1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256, 4.096, 65.536,
1.048.576`.

Suma, producto, cuadrado y round-trip conservan toda la curva. Inversión queda
acotada a 4.096 elementos: el primer preflight mostró que extenderla hasta un
millón bajo 30 procesos y 60 acciones por proceso produciría decenas de horas
por celdas concretas sin añadir una frontera distinta. La operación no se
elimina; se mide completa dentro de su rango declarado.

Resultado generado:

| Shard | Operaciones | Celdas publicables | Preflight |
|---|---:|---:|---:|
| F1 campos binarios | 15 | 249 | 15 |
| F2 campos primos | 15 | 249 | 15 |
| Total | 30 | 498 | 30 |

El 1 % de 498 serían cinco celdas, insuficientes para probar 30 operaciones.
P0 conserva una por operación y registra la fracción real, 6,02 %, en vez de
presentar una muestra pequeña como cobertura suficiente.

## Ejecución observada

Los runs válidos son:

- `c3-p0-f1-static-binary-preflight-v2`;
- `c3-p0-f2-static-prime-preflight-v2`.

Se ejecutaron 60 procesos y 300 observaciones. Las 30 celdas completaron sin
fallos, 23 alcanzaron el umbral relajado de Smoke y siete quedaron imprecisas
con dos procesos. Ningún worker mostró checksums distintos entre observaciones
del mismo proceso. El tiempo cronometrado agregado fue 0,715 s y los artefactos
ocupan 160.140 bytes.

La ejecución fue `Smoke` sobre un árbol deliberadamente sucio mientras se
implementaba P0, en un Intel i7-13700HX. No se interpreta la precisión ni los
tiempos como resultados de rendimiento del producto.

## Proyección inicial

Para las 498 celdas F1/F2 y el mínimo de 30 procesos hay 14.940 workers por
host. Extrapolando el coste por elemento observado, 10 warmups y 50 muestras,
el tramo cronometrado se estima en 3,64 host-horas. Con calibración, setup y
variación se reserva inicialmente una ventana de 4–8 horas por host al mínimo
de procesos y 12–25 horas si muchas celdas alcanzan 100 procesos.

El raw F1/F2 se estima en 150–300 MiB por host con 50 observaciones. Esta
proyección no cubre todavía suites de DB, grafos exactos, corpus o soak; su
coste se añadirá shard por shard, no se extrapolará desde aritmética de campo.

## Correcciones realizadas durante P0

1. Se descartó el primer run porque los inputs de campo mutaban entre acciones.
2. Se hizo inmutable el input y reutilizable la salida.
3. Se hizo depender el checksum de todos los elementos.
4. Se eliminó la ruta del checkout del informe reproducible.
5. Se introdujo `maximum_scale` por operación.
6. Se evitó una muestra nominal del 1 % que no cubría cada operación.
7. Se añadió un test que falla si el inventario declara implementada una
   operación que el worker no registra.

Los runs descartados se movieron fuera del repositorio a
`/tmp/algesum-c3-p0-discarded-stateful-v1`; no forman parte de la evidencia.

## Estado del resto de C3

P0 no oculta las brechas. `c3-operation-inventory-v1.json` marca F1–X2 como
parciales o externos y enumera cada workload ausente. Los siguientes shards
deben implementarse en este orden:

1. F3/F4: batch, packed, algoritmos derivados, runtime y generación;
2. S1–S3: firmas base/multievaluación, tracked, snapshots, wires y deltas;
3. T1/R1/D1: árboles, archivos, reconciliación y DB en memoria;
4. G1/G2/X1/X2: grafos, exacto/DAG, corpus, wires y packaging;
5. D2: PostgreSQL, logical decoding, concurrencia, recovery y soak.

Cada incorporación debe actualizar el inventario de `missing` a
`implemented`, generar su shard, ejecutar su preflight y publicar un commit
verde. La campaña `Controlled` no comienza hasta que el inventario deje de
tener huecos de implementación.
