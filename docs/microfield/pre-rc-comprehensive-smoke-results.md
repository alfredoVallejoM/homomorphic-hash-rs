# Resultados de la campaña integral C1

Fecha: 2026-08-10. Clasificación: **Smoke/Informative**. `claims_allowed=false`.
Las firmas medidas son resúmenes algebraicos homomórficos no criptográficos.

## Resultado global

El smoke transversal ejecutó 65 celdas, 25 comparaciones emparejadas y 27
curvas. Las 65 celdas cumplieron el umbral de precisión relajado de C1. Esto
valida cobertura, preparación, ejecución y agregación; no convierte las cifras
en resultados publicables.

## Firmas algebraicas

Tiempos medianos por acción observados con 4.096 elementos de 16 bytes:

| Ley | Build total | Composición de dos resúmenes |
|---|---:|---:|
| aditiva | 790,8 µs | 7,3 ns |
| secuencia | 755,2 µs | 392,0 ns |
| bidireccional | 842,0 µs | 7,2 ns |
| multiconjunto | 767,3 µs | 8,7 ns |
| multiconjunto K=2 | 835,9 µs | 9,2 ns |
| secuencia K=2 | 750,2 µs | 702,1 ns |

La hipótesis básica queda confirmada: construir depende del volumen ya
resumido mientras que componer depende del número de lanes/evaluaciones, no de
los 4.096 elementos originales. Secuencia y secuencia K=2 pagan además la
combinación de longitudes/potencias.

Con payloads de 1.024 bytes, 4.096 elementos tardaron aproximadamente 23 ms
tanto en la firma aditiva como en la secuencia K=2. La codificación del payload
domina y el coste de lanes adicionales queda oculto. El contador observó una
asignación aproximada por payload en esa ruta: es el primer objetivo concreto
de optimización antes de C2.

Estas razones son señales del smoke, no speedups publicables. C2 debe ampliar
N, payload, fragmentos y K, y añadir baselines que cambien datos entre acciones
para descartar cualquier simplificación excesiva del compilador.

## Grafos

La firma rápida preparada produjo:

| Vértices | ns/vértice | Tiempo total aproximado |
|---:|---:|---:|
| 256 | 616,2 | 0,16 ms |
| 16.384 | 509,1 | 8,34 ms |
| 131.072 | 503,1 | 65,95 ms |

La estabilización alrededor de 0,50 µs/vértice es compatible con crecimiento
lineal para el ciclo disperso medido. Falta variar grado, densidad, etiquetas y
familia antes de generalizar.

En caminos pequeños de 6–14 vértices, reutilizar el DAG costó entre 0,76x y
0,99x la canonización exacta. El baseline es demasiado fácil y las asignaciones
siguen siendo elevadas, por lo que el DAG no muestra todavía su ventaja
esperada. C2 debe medir grafos exactos adversariales, múltiples resoluciones y
updates reales del DAG; no basta con repetir un camino discreto.

## PostgreSQL: instrumentación corregida

El laboratorio v2 descubrió que preparaba una copia autoritativa completa antes
de cada aplicación aunque la ruta incremental no la solicitara. La copia estaba
fuera del cronómetro, pero contaminaba caché y memoria. Se convirtió en lazy y
solo se materializa si se selecciona un rebuild autoritativo.

La celda de 256 cambios sobre un millón pasó de 193,8 ms sesgados a 8,5 ms. La
evidencia anterior a esta corrección queda supersedida para rendimiento, aunque
sus comprobaciones de exactitud siguen siendo válidas.

### 65.536 filas, strided, tres repeticiones

| Cambios | PostgreSQL | Algesum | Ruta | Rebuild de referencia |
|---:|---:|---:|---|---:|
| 256 | 7,3 ms | 6,3 ms | incremental | 167,4 ms |
| 4.096 | 49,9 ms | 31,1 ms | incremental | 158,6 ms |
| 16.384 | 133,7 ms | 99,3 ms | incremental | 157,5 ms |
| 32.768 | 217,4 ms | 189,7 ms | incremental | 158,0 ms |
| 49.152 | 342,2 ms | 257,8 ms | rebuild/híbrida por partición | 158,7 ms |
| 65.536 | 426,2 ms | 289,4 ms | rebuild por partición | 158,5 ms |

### Un millón de filas, una repetición de capacidad

| Distribución | Cambios | PostgreSQL | Algesum | Ruta |
|---|---:|---:|---:|---|
| strided | 256 | 15,5 ms | 8,5 ms | incremental |
| clustered | 256 | 7,1 ms | 8,1 ms | incremental |
| hotspot | 256 | 8,4 ms | 11,9 ms | incremental |
| strided | 10.000 | 87,3 ms | 95,2 ms | incremental |
| clustered | 10.000 | 102,1 ms | 92,0 ms | incremental |
| strided | 250.000 | 2,29 s | 1,54 s | incremental |
| clustered | 250.000 | 1,95 s | 1,55 s | incremental |
| strided | 750.000 | 5,88 s | 4,47 s | rebuild por partición |
| clustered | 750.000 | 6,31 s | 4,44 s | rebuild por partición |
| strided | 1.000.000 | 6,81 s | 4,91 s | rebuild por partición |
| clustered | 1.000.000 | 8,78 s | 4,57 s | rebuild por partición |

Todos los casos conservaron filas exactas y resumen igual al rebuild. La ruta
sparse depende principalmente del lote y de las particiones tocadas, no del
tamaño total: 256 cambios cuestan 6,3 ms con 65k filas y 8,1–11,9 ms con un
millón. A alta densidad, la distribución modifica mucho más PostgreSQL que
Algesum; el particionado por clave amortigua la localidad externa.

La creación inicial de Algesum pasó de 166,8 ms para 65.536 filas a 3,05 s para
un millón. El proceso completo de un millón con cinco densidades alcanzó
1.504.616 KiB de RSS, cero swaps propios y 54,07 s de pared. Un lote superior a
100.000 requiere elevar explícitamente los límites del laboratorio; los
defaults de producción no cambiaron.

El preflight de 10 millones se detuvo antes de ejecutar: la extrapolación de
memoria quedaba demasiado cerca de la memoria disponible y habría introducido
swap. Se necesita un host dedicado con al menos 32 GiB libres para obtener una
medición válida.

## Decisión para C2

La campaña integral puede avanzar a piloto, con cuatro correcciones de diseño:

1. eliminar asignaciones por payload en firmas o medirlas explícitamente;
2. añadir composición con datos alternantes y K=1..4;
3. sustituir los caminos exactos fáciles por familias y pares adversariales;
4. ejecutar PostgreSQL con tres o más repeticiones en un millón y reservar 10 M
   para un host con memoria suficiente.

Los resultados crudos viven en
`validation/benchmarks/runs/pre-rc-comprehensive-smoke-v1` y en los informes
`pre-rc-postgresql-*-post-lazy-v2.json`.
