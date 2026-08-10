# Resultados de la campaña integral C2

Fecha: 2026-08-10. Clasificación: **Pilot/Informative**.
`claims_allowed=false`. Las firmas medidas son resúmenes algebraicos
homomórficos **no criptográficos**.

## Resultado global

C2 ejecutó dos campañas reproducibles de firmas y grafos: 47 celdas, 470
procesos independientes y 7.050 observaciones. 44/47 celdas alcanzaron el IC
relativo de mediana <= 10 %. Las tres restantes fueron dos composiciones de
multiconjunto extremadamente cortas (6,5 y 8,7 ns) y el ciclo exacto de 12
vértices (10,08 %); son resultados estadísticamente inconclusos, no fallos de
corrección.

La integración PostgreSQL añadió 33 muestras verificadas sobre un millón de
filas, tres distribuciones y lotes de hasta un millón. Todas conservaron las
filas exactas y un resumen idéntico al rebuild.

## Firmas K=1..4

Con 4.096 elementos de 16 bytes, aumentar evaluaciones afecta poco al build
porque domina la codificación del payload:

| Ley | K=1 | K=2 | K=3 | K=4 |
|---|---:|---:|---:|---:|
| multiconjunto, build 64 | 12,26 µs | 12,60 µs | 12,63 µs | 14,66 µs |
| secuencia, build 4.096 | 749,9 µs | 747,8 µs | 752,4 µs | 787,6 µs |
| secuencia, composición | 334,9 ns | 653,9 ns | 987,5 ns | 1.296,2 ns |

La composición de secuencia crece aproximadamente por lane, como se esperaba.
La composición K=4 sigue siendo unas 607 veces menor que su build emparejado,
pero esta razón es calibración interna y no un speedup publicable.

Para K=4 y 4.096 elementos, pasar de payloads de 8 a 1.024 bytes elevó el build
de 0,61–0,73 ms a aproximadamente 23,0 ms en ambas leyes. El coste está
dominado por bytes codificados, no por elegir multiconjunto o secuencia.

### Fragmentación y datos alternantes

La subcampaña compuso resúmenes K=4 previamente construidos sobre 65.536
elementos. Cada acción alternó entre dos juegos de operandos y los pasó por
`black_box` para impedir que un único resultado constante explicase la cifra.

| Fragmentos | Multiconjunto K=4 | Secuencia K=4 |
|---:|---:|---:|
| 2 | 47 ns | 1,23 µs |
| 8 | 271 ns | 9,11 µs |
| 64 | 2,31 µs | 82,26 µs |
| 1.024 | 37,70 µs | 1,334 ms |

Las 8/8 celdas fueron precisas. El coste es aproximadamente lineal en el número
de fragmentos; la secuencia paga además longitud y potencias por cada
concatenación. El protocolo exige cinco puntos para estimar formalmente una
pendiente y esta calibración tiene cuatro, por lo que el agregado conserva
`InsufficientScalePoints` y no fabrica una pendiente.

La extensión v3 añadió el quinto punto (256 fragmentos), repitió cada celda en
10 procesos independientes y obtuvo 10/10 celdas precisas. Las pendientes
log-log ya estimables son 1,0645 para multiconjunto y 1,1011 para secuencia:
confirman crecimiento aproximadamente lineal en fragmentos, sin convertirlo en
un claim público (`Informative`, `claims_allowed=false`).

## Grafos

La firma rápida preparada se mantuvo estable al crecer de 4.096 a 131.072
vértices:

| Familia | 4.096 | 131.072 |
|---|---:|---:|
| ciclo | 501,5 ns/vértice | 501,9 ns/vértice |
| estrella | 491,9 ns/vértice | 496,9 ns/vértice |
| regular grado 8 | 1.131,2 ns/vértice | 1.150,4 ns/vértice |

El grado tiene un impacto observable: el regular grado 8 cuesta unas 2,3 veces
el ciclo/estrella, mientras el coste por vértice permanece estable en las dos
escalas.

Una modificación de etiqueta sobre un regular grado 8 obtuvo estas razones
emparejadas incremental/reanálisis completo:

| Vértices | Ratio | IC 95 % | Ventaja aproximada |
|---:|---:|---:|---:|
| 1.024 | 0,3496 | [0,3403, 0,3543] | 2,86x |
| 16.384 | 0,3381 | [0,3373, 0,3463] | 2,96x |
| 131.072 | 0,3588 | [0,3498, 0,3702] | 2,79x |

Los caminos exactos y ciclos simétricos de 8–16 vértices ejecutaron con un
presupuesto de un millón de nodos. `Inconclusive` se conserva como salida
válida en lugar de abortar o presentarse como igualdad exacta. Antes de C3 el
informe agregado debe exponer además el resultado exacto/inconcluso como campo
estructurado, no sólo mediante checksum.

Ese bloqueo quedó cerrado en la campaña de telemetría posterior a C2. Los
workers, el agregado JSON, el CSV y el informe Markdown exponen ahora outcome,
presupuesto, nodos explorados, hojas, profundidad, ruta y límite agotado. Los
30 workers fueron coherentes: caminos de 8/16 vértices `exact` por
`ExactRefinementDiscrete` con 0 nodos de búsqueda; ciclos de 8/12/16 `exact`
por `IndividualizationRefinement` con 7 nodos, 4 hojas y profundidad 2. La
sexta celda forzó un presupuesto de un nodo y registró correctamente
`inconclusive` y `search-nodes`, sin publicar una forma. La clasificación sigue
siendo `Informative` y `claims_allowed=false`.

## PostgreSQL a un millón de filas

Medianas de tres repeticiones; `Algesum` mide la aplicación y `rebuild` la
reconstrucción completa de referencia:

| Distribución | Cambios | PostgreSQL | Algesum | Rebuild | Ruta |
|---|---:|---:|---:|---:|---|
| strided | 256 | 9,7 ms | 8,3 ms | 2,78 s | incremental |
| strided | 10.000 | 88,1 ms | 94,3 ms | 2,78 s | incremental |
| strided | 250.000 | 2,15 s | 1,54 s | 2,77 s | incremental |
| strided | 750.000 | 6,12 s | 4,72 s | 2,88 s | particiones |
| strided | 1.000.000 | 7,05 s | 4,96 s | 2,76 s | particiones |
| clustered | 256 | 9,2 ms | 9,5 ms | 2,79 s | incremental |
| clustered | 250.000 | 1,86 s | 1,53 s | 2,77 s | incremental |
| clustered | 1.000.000 | 9,60 s | 5,07 s | 2,77 s | particiones |
| hotspot | 256 | 8,2 ms | 8,6 ms | 2,74 s | incremental |
| hotspot | 4.096 | 38,7 ms | 56,2 ms | 2,75 s | incremental |
| hotspot | 62.500 | 504,1 ms | 406,4 ms | 2,77 s | incremental |

No existe una degradación problemática en 256: el coste permanece en 8–10 ms
sobre un millón de filas. La frontera real está a alta densidad. A 250.000
cambios el incremental aún gana al rebuild; a 750.000 y un millón, la política
elige particiones pero tarda 1,64–1,80 veces el rebuild completo. Es un defecto
de selección de estrategia, no de exactitud.

## Decisión y siguiente campaña

C2 cierra como calibración informativa. Antes de congelar C3 deben completarse:

1. ~~añadir `FullRebuild` al selector adaptativo de DB y medir el cruce entre
   25 % y 75 % de densidad~~: cerrado; la ruta global perdió el A/B y queda
   opt-in, mientras la ruta por particiones mejoró 15-16 % a alta densidad;
2. ~~registrar outcome y presupuesto exacto de grafos como métricas
   estructuradas~~: cerrado y verificado en 30 workers independientes;
3. ~~ampliar grafos con topología incremental, mallas, densidad y corpus
   externo~~: piloto sintético preciso y cuatro suites externas verdes; sólo
   quedan los tiempos externos en host C3;
4. ~~añadir el quinto punto de fragmentación y conservar operandos
   alternantes~~: cerrado con 10/10 celdas precisas y pendientes estimadas;
5. ejecutar WAL/logical decoding, reinicio y concurrencia como campaña de
   integración separada;
6. replicar las celdas discriminantes de todas las estructuras en host
   dedicado `Controlled` antes de cualquier claim o integración RC.

Los datos crudos están en
`validation/benchmarks/runs/pre-rc-comprehensive-pilot-v1`,
`pre-rc-comprehensive-fragmentation-pilot-v1`,
`pre-rc-comprehensive-fragmentation-pilot-v3`,
`pre-rc-graph-exact-telemetry-pilot-v4`,
`pre-rc-graph-topology-density-pilot-v1` y los informes PostgreSQL descritos en
la sección correspondiente.

## Resultados posteriores a C2: grafos y corpus externo

La campaña de topología/densidad añadió mallas, regulares de grado 8/32 y una
edición real de arista frente a reanálisis completo. Sus 10 celdas y 50 workers
fueron precisos:

| Familia/ruta | 4.096 vértices | 16.384 vértices |
|---|---:|---:|
| malla, firma preparada | 680,1 ns/vértice | 680,0 ns/vértice |
| regular grado 8 | 1.257,1 ns/vértice | 1.263,9 ns/vértice |
| regular grado 32 | 3.341,3 ns/vértice | 3.338,8 ns/vértice |
| reanálisis topológico completo | — | 25,87 ms |
| actualización topológica incremental | — | 9,26 ms |

La razón incremental/completa fue 0,4078 a 1.024 vértices y 0,3597 a 16.384,
equivalente a aproximadamente 2,45x y 2,78x en esta calibración informativa.

El corpus externo fijado se verificó offline contra sus SHA-256 y pasó sus
cuatro suites opt-in: NetworkX Graph Atlas, TUDataset MUTAG, SNAP Email-EU y XGI
Diseasome. Cubren pares no isomorfos del atlas, moléculas etiquetadas, red
dirigida etiquetada e hipergrafo con roles. Esto cierra cobertura semántica del
corpus; sus tiempos sólo se medirán en la campaña C3 controlada.
