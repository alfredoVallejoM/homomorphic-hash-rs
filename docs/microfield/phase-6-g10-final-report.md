# Informe de cierre — F6.G10, motor Microcanon compacto

Fecha: 3 de agosto de 2026.

Estado: completado localmente; pendiente de commit y CI remoto.

## Resultado

`MicrocanonStrategy::Compact` es ahora la estrategia predeterminada. G9 se
conserva como `MicrocanonStrategy::Reference`: no interviene en producción,
pero permite comparar cada byte y detectar regresiones semánticas. Ambas rutas
dependen solo de `IncidenceGraph` y `GraphSchemaId`; ninguna firma finita,
campo, encoder, lane o capability ISA puede decidir la forma exacta.

G10 no cambia el wire `MFC2`, el parser ni la definición de igualdad. Antes de
publicar una forma, la fachada vuelve a parsearla y verifica el mapping sobre
todos los vértices e incidencias. Un límite agotado produce `Inconclusive` y no
expone un candidato parcial.

## Refinamiento compacto

El refinador reemplaza las claves G9 `Vec<Vec<u8>>` por:

- una arena plana de exactamente dos entradas por incidencia normalizada,
  una para CSR saliente y otra para CSR entrante;
- rangos `start + len` por vértice;
- ranks canónicos preparados una vez para labels y descriptores;
- tuples enteras de color vecino, tamaño de descriptor, rank y multiplicidad;
- buffers reutilizables de keys, orden, colores y tamaños de celda.

El orden entero reproduce exactamente el framing G9. En particular, el
contenido de cada incidencia se ordena primero y su longitud enmarcada se
compara después al ordenar claves de vértice. Este matiz está cubierto por 128
grafos relacionales deterministas con labels, relaciones, roles,
multiplicidades, dirección y longitudes variables.

`MicrocanonWorkspace` permite reservar y reutilizar la memoria O(V+I) del
refinador. La búsqueda conserva asignaciones por estado DFS y por forma final;
la garantía no se presenta como “cero asignaciones totales”. Tras preparar el
workspace no existe una asignación independiente por incidencia o por clave de
vértice durante las rondas.

## Búsqueda y podas certificadas

La búsqueda es DFS iterativa y mantiene el camino individualizado de cada
frame. Sus dos podas activas son:

1. **órbitas del estabilizador**: cuando dos hojas producen la misma forma, se
   deriva una permutación y `VerifiedGraphMapping` comprueba que sea un
   automorfismo real. Solo los generadores que fijan punto a punto el camino
   actual pueden unir candidatos de una órbita y eliminar un hijo;
2. **prefijo de vértices**: en una hoja discreta se compara primero la sección
   completa de vértices con el incumbent. Si ya es lexicográficamente mayor,
   ningún byte posterior de incidencias puede mejorar la forma.

La traza compacta registra eventos y un checksum de particiones para auditoría.
No se usa como justificación de poda. El selector de celda permanece deliberada
y canónicamente igual a G9: menor celda no unitaria y, en empate, menor color.
Un experimento adaptativo alteró bytes exactos y fue retirado. Estabilidad y
corrección tienen prioridad sobre una heurística no certificada.

## Presupuestos fail-closed

`CanonicalSearchBudget` controla ahora:

- nodos de búsqueda;
- celdas `usize` retenidas;
- bytes retenidos contabilizados;
- profundidad de individualización;
- deadline cooperativo de pared.

`CanonicalBudgetLimit` identifica `SearchNodes`, `RetainedStateCells`,
`RetainedBytes`, `SearchDepth` o `ElapsedTime`. El deadline se comprueba tras la
preparación atómica, en cada ronda y nodo, y nuevamente después de verificar el
resultado. El presupuesto de bytes incluye arenas, ranks, colores, stack,
automorfismos, órbitas, formas/mappings y artefactos canónicos de componentes.
La referencia G9 coopera con nodos/frontier y aplica profundidad/bytes antes de
publicar, pero puede detectarlos al final; solo G10 se recomienda para parada
temprana de esos dos límites.

`peak_tracked_bytes` es una medida lógica conservadora de los buffers
controlados, no RSS. Excluye el grafo inmutable de entrada, metadata del
allocator, el objeto de salida una vez transferido al caller y ciertos
temporales atómicos de construcción/verificación. La documentación y la API no
lo presentan como una cota del proceso completo.

## Evidencia de corrección

La batería G10 añade:

- G10 contra G9 y contra un oráculo factorial independiente en todos los grafos
  simples hasta cinco vértices;
- gate release de los 32.768 grafos simples etiquetados de orden seis: 156
  clases en G9, G10 y el oráculo, sin colisiones ni divergencias;
- 128 grafos relacionales pseudoaleatorios deterministas y sus renumeraciones;
- dirección, labels, roles, multiplicidad, hiperaristas y componentes;
- verificación de automorfismos y mappings positivos/negativos;
- agotamiento independiente de bytes, profundidad, tiempo y nodos;
- contabilización específica de formas retenidas por componentes;
- reutilización estable del workspace y rechazo tipado con la estrategia G9;
- equivalencia con perfiles F251/GF(2^256), encoders y lanes distintos.

El grafo de aceptación `C32` recorre 97 nodos en G9 y 7 en G10: 92,8 % menos.
La forma exacta publicada es idéntica.

## Medición Criterion local

Build `release`, LTO del repositorio, diez muestras, misma máquina y mismo
proceso. Los intervalos observados fueron:

| Caso | G10 compacto | G9 referencia | Mejora aproximada |
|---|---:|---:|---:|
| ciclo 6 | 12,50–12,53 µs | 167,53–167,85 µs | 13,4× |
| ciclo 8 | 20,56–20,62 µs | 420,50–422,19 µs | 20,5× |
| ciclo 10 | 26,37–26,43 µs | 898,49–901,32 µs | 34,1× |
| ciclo 12 | 35,90–36,19 µs | 1,603–1,625 ms | 44,8× |
| discreto 64 | 46,17–46,20 µs | 93,15–94,28 µs | 2,0× |
| discreto 256 | 171,90–173,92 µs | 378,27–380,78 µs | 2,2× |

No existe regresión del 5 % en la ruta discreta: G10 es aproximadamente dos
veces más rápido en los dos tamaños medidos. Estas cifras son evidencia local,
no una promesa multi-CPU ni una comparación todavía con nauty/Traces.

## Decisiones revisadas y límites

El plan proponía active-cell refinement, radix/counting sort, selector
adaptativo y trace pruning. No se promocionaron solo por aparecer en el plan:

- las arenas, ranks y tuples enteras con `sort_unstable` ya superan los gates;
- active-cell y radix permanecen candidatos si demuestran mejor ensamblado y
  bytes idénticos en todo el corpus relacional;
- el selector adaptativo se rechazó al cambiar la identidad exacta G9;
- la traza no poda hasta disponer de una regla probada y diferencialmente
  aislable.

`MicrocanonStrategy::Reference` proporciona el modo completo sin
optimizaciones. G10 no ofrece aún switches públicos para desactivar cada poda
por separado. Tampoco implementa árboles/bloques, matcher pareado, 2-WL
localizado ni firmas de loops; corresponden a G11–G13.

## Siguiente paso

F6.G11 debe reforzar las firmas sin convertirlas en autoridad exacta:
assurance explícito, encodings de lane realmente independientes, secuencias
multievaluadas, moments por celda y el catálogo relacional L0–L3. Cada canal
necesita ley, wire, contraejemplo, metamorfismo y benchmark antes de alimentar
el routing o los node invariants de Microcanon.
