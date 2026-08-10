# C3 G1/G2: grafos, exactitud y DAG

Fecha: 2026-08-11. Estado: **implementado y ejecutado como Smoke;
`Controlled` pendiente**.

Los canales de campo finito y sus resúmenes son huellas algebraicas
**no criptográficas**. Solo la canonización y la comparación completa de bytes
proporcionan identidad exacta dentro del contrato de grafos.

## Cobertura cerrada

El plan `c3-g1-g2-factor-plan-v1.json` añade 126 celdas publicables y eleva el
inventario C3 generado a 3.110. Se separan cinco preguntas para que los cruces
sean interpretables:

- construcción/preparación, campo preparado, canal híbrido, paralelo y memoria;
- lotes incrementales de labels frente al tamaño total del grafo;
- presupuestos de nodos, celdas retenidas, bytes retenidos y profundidad;
- path, cycle, componentes desconectados, regular y star;
- persistencia MFGD y recanonización del DAG después de un `GraphDelta`.

La expansión también corrigió un defecto general: un shard cuyo tamaño mínimo
era mayor que uno hacía que el muestreo Smoke dividiera por cero. El selector
usa ahora la celda mínima disponible de cada operación. El analizador acepta
telemetría exacta para las matrices nuevas, no solo para el identificador
histórico `graph.exact`.

## Resultados del preflight

Los cinco preflights completaron 17 celdas, 34 workers y 170 observaciones.
Todas las celdas cumplieron el umbral Smoke del 25 %, con una semianchura
relativa mediana de 1,05 % y máxima de 10,87 %. Los checksums fueron estables.

En 64 vértices, el análisis de campo preparado fue allocation-free y obtuvo
508,58 ns/vértice. El canal híbrido campo+SHA costó 2.175,77 ns/vértice, unas
4,28 veces más. Forzar paralelismo en ese tamaño costó 4.682,80 ns/vértice:
es una confirmación directa de que el umbral conservador debe mantenerse y
que el paralelismo solo se puede juzgar en las escalas grandes publicables.

La actualización transaccional de un label sobre 1.024 vértices costó una
mediana de 587.271 ns por edición y alcanzó un pico de 454.011 bytes. El camino
incremental evita recomputar ciegamente todas las rondas, pero todavía paga la
preparación de un reemplazo inmutable; los lotes son por tanto esenciales para
amortizar el coste fijo.

## Exactitud y simetría

Los límites se comportaron fail-closed: un nodo de búsqueda terminó como
`inconclusive/search-nodes`, y un byte retenido como
`inconclusive/retained-bytes`. Los presupuestos suficientes produjeron formas
exactas. Esto valida el significado de los límites, no su calibración final.

El tamaño aislado no predijo el coste. Un path de seis vértices quedó discreto
sin búsqueda; un ciclo necesitó 7 nodos; dos componentes cíclicos, 14; y un
star de diez vértices llegó a 129 nodos y profundidad 8, con 41.593,55
ns/vértice. La campaña publicable debe estratificar por familia de simetría y
reportar nodos, hojas, profundidad, límite agotado y memoria junto al tiempo.

Persistir/restaurar el DAG y recanonizarlo después de un delta fueron precisos
y deterministas. El Smoke no convierte los fingerprints en prueba de
isomorfismo: la reutilización del DAG continúa exigiendo bytes canónicos
exactos.

## Siguiente bloque

G1 y G2 ya no tienen operaciones pendientes. Sigue X1 con corpora externos y
dominios de química/lógica/red, después X2 con familias wire, fallos,
consumidor de paquete y fachada legacy. D2 cerrará los escenarios PostgreSQL.
