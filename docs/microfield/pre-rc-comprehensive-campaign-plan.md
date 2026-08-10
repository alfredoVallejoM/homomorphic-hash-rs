# Campaña integral pre-RC de Algesum

Fecha: 2026-08-10. Estado: **C2 ejecutado; C3 pendiente**. Clasificación inicial:
**Informative**; ninguna medición habilita por sí sola afirmaciones públicas.

Algesum mide resúmenes y firmas algebraicas homomórficas **no criptográficas**.
La campaña no interpreta igualdad de firmas como prueba de igualdad ni añade
propiedades de resistencia a colisiones.

## Objetivo

Cerrar la RC con evidencia reproducible en cuatro niveles:

1. corrección semántica y modelos de referencia;
2. capacidad y límites explícitos;
3. rendimiento, memoria y escalado;
4. integración, reinicio y comportamiento bajo carga.

El arnés estadístico existente sigue siendo la única fuente de resultados de
rendimiento. Conserva procesos aislados, orden aleatorio determinista,
percentiles, bootstrap, asignaciones, clasificación del entorno y checksums.

## Niveles de ejecución

| Nivel | Propósito | Procesos/celda | Resultado |
|---|---|---:|---|
| C0 | leyes, modelos, wires, fallos y fuzzing | no aplica | puerta de corrección |
| C1 | smoke de cobertura integral | 2 | detecta rutas rotas |
| C2 | piloto informativo | 10 | calibra escalas y fronteras |
| C3 | campaña controlada | 30–50 adaptativos | evidencia publicable |

C3 solo puede ejecutarse en host dedicado, frecuencia y afinidad controladas,
árbol limpio y binario de release identificado. C1 y C2 nunca permiten claims.

## Inventario y matriz transversal

| Familia | Corrección | Escalas prioritarias | Operaciones principales |
|---|---|---|---|
| Campos binarios y primos | leyes, referencia, ISA diferencial | 1–1.048.576 elementos | suma, producto, cuadrado, inversa, batch, packed |
| Firmas algebraicas | ley de composición y rebuild exacto | 0–1.048.576 elementos | construir, componer, retirar, serializar, delta |
| Árboles de resumen | edición = rebuild | 4 KiB–1 GiB | edición local, lote, híbrida, rebuild |
| Base de datos | filas y resumen exactos | 65.536–10 M filas | transacción, partición, checkpoint, replay |
| Reconciliación | diferencia recuperada exacta | diferencia 0–64 | sketch, merge, decode, límites |
| Grafos | invariancia y canon exacto cuando procede | 8–1 M vértices | fast, incremental, global, exacto, DAG |
| Generación dinámica | artefacto reproducible | grados 8–512 | parse, plan, generar, verificar, cargar |
| Wires y snapshots | round-trip/corrupción/versiones | pequeño–límite | encode, decode, rechazo, restart |

Las escalas extremas se someten primero a un preflight de memoria. Un fallo del
preflight se registra como límite de capacidad; no se sustituye por swapping ni
por una medición incomparable.

## Firmas homomórficas: campaña prioritaria

### Leyes y perfiles

- aditiva: combinación conmutativa;
- secuencia: concatenación ordenada;
- secuencia bidireccional: concatenación y orientación;
- multiconjunto: combinación con multiplicidad;
- multievaluación de multiconjunto, `K = 1, 2, 3, 4`;
- multievaluación de secuencia, `K = 1, 2, 3, 4`;
- campos estáticos y campos dinámicos compatibles.

Para cada perfil se compara composición de dos y 2^k fragmentos frente a una
reconstrucción sobre los datos concatenados/unidos. La igualdad debe ser del
100 % en todos los casos válidos. También se prueban contexto, encoder, campo,
bases y puntos de evaluación incompatibles: todos deben fallar de forma
cerrada.

### Ejes de rendimiento

- elementos: `0, 1, 8, 64, 512, 4.096, 65.536, 1.048.576`;
- payload: `0, 8, 16, 64, 1.024, 65.536` bytes;
- fragmentos: `1, 2, 8, 64, 1.024`;
- operación: build, composición, append/insert, remove/trim, delta y wire;
- memoria: asignaciones, bytes totales y pico;
- salida: ns/elemento para build y ns/composición para la ley homomórfica.

La hipótesis principal es que build crece con los bytes procesados y la
composición permanece independiente del tamaño lógico ya resumido. La campaña
debe informar el punto en que codificar el payload domina el coste de campo.

## Grafos: campaña prioritaria

### Familias de entrada

- caminos, ciclos, estrellas, mallas y grafos regulares;
- Erdős–Rényi dispersos/densos y Barabási–Albert;
- etiquetas homogéneas, únicas y Zipf;
- componentes desconectados, multiaristas, lazos e hiperaristas;
- pares isomorfos relabelados y pares cercanos no isomorfos;
- corpus externos y casos adversariales de refinamiento/canonización.

### Rutas medidas

1. construcción CSR y preparación;
2. firma rápida preparada, end-to-end y paralela;
3. actualización incremental de etiqueta y topología frente a reanálisis;
4. invariantes globales, momentos, matrices, walks y patrones;
5. filtro adaptativo y escalada;
6. canonización exacta con presupuesto explícito;
7. reutilización y actualización del DAG canónico.

Escalas rápidas: `256, 4.096, 16.384, 131.072, 1.048.576` vértices, con grados
medios `2, 8, 32, 128`. Canonización exacta usa `6, 8, 10, 12, 14, 16` y
presupuestos crecientes. Un resultado inconcluso es una salida válida que debe
registrar presupuesto consumido; nunca se convierte en igualdad exacta.

Métricas: ns/vértice, ns/incidencia/ronda, speedup paralelo, memoria pico,
vértices auditados por delta, tasa de escalada, nodos/estados exactos y tasa de
reutilización del DAG. La corrección exige invariancia bajo relabeling y
coincidencia con el oráculo exacto donde el presupuesto termina.

## Base de datos e integración PostgreSQL

El laboratorio v2 registra por separado creación PostgreSQL, carga inicial,
construcción Algesum, commit, aplicación, rebuild y verificación. Incluye tres
distribuciones: `clustered`, `strided` y `hotspot`.

### Matriz de escalado

| Eje | Valores |
|---|---|
| filas | 65.536, 1 M, 10 M |
| modificaciones | 1, 256, 4.096, 1 %, 25 %, 50 %, 75 %, 100 % |
| distribución | clustered, strided, hotspot 1/16 |
| particiones | 64, 256, 1.024, 4.096 |
| clientes PostgreSQL | 1, 16, 64, 256 |
| entrega | directa, redelivery, WAL logical decoding |
| recuperación | checkpoint limpio, crash antes/después de commit, restart |

65.536 y 1 M forman el piloto. 10 M solo pasa a medición tras el preflight de
memoria y tiempo, y se ejecuta con menos densidades antes de expandirse. La
concurrencia mide PostgreSQL y la cola de ingestión; la publicación de estado
Algesum sigue un orden total por fuente y debe aplicar backpressure explícita.

Puertas: igualdad exacta en cada commit, resumen igual a rebuild, cero commits
perdidos/duplicados, checkpoint recuperable, posición monótona, p99 y backlog
acotados, y memoria pico documentada.

## Diseño experimental y criterios

- Semillas y corpus quedan versionados.
- Cada comparación usa procesos emparejados y el mismo host.
- El piloto acepta CI relativa de mediana <= 10 %; C3 exige <= 5 %.
- C3 informa mediana, p95, p99, MAD, CI95 y asignaciones.
- Toda ruta optimizada tiene baseline semánticamente equivalente.
- Las referencias que omiten validación transaccional se etiquetan como suelo
  algorítmico, nunca como competidor directo.
- Regresión mayor del 5 % en rutas congeladas bloquea la RC salvo explicación y
  nueva decisión versionada.
- Los resultados de grafos se estratifican por familia; no se promedian grafos
  fáciles y adversariales en una única cifra.

## Artefactos lanzados

- `comprehensive-smoke-v1.json`: C1 para todas las familias, con curvas amplias
  de firmas y grafos;
- nuevas operaciones de composición constante para las seis firmas mantenidas;
- laboratorio PostgreSQL `algesum-postgresql-scaling-v2`, con distribuciones y
  tiempos de preparación/verificación separados.
- `comprehensive-pilot-v1.json`: K=1..4, payloads, familias rápidas, edición
  incremental y canon exacto presupuestado;
- `comprehensive-fragmentation-pilot-v1.json`: 2–1.024 fragmentos K=4 con dos
  juegos de operandos alternantes;
- tres campañas PostgreSQL C2 de un millón de filas y tres repeticiones por
  celda para `clustered`, `strided` y `hotspot`.

C1 y C2 están ejecutados. Sus resultados y fronteras están en
`pre-rc-comprehensive-pilot-results.md`; después de corregir el selector denso
de DB y la telemetría exacta se seleccionarán sólo las celdas discriminantes
para C3. Esto evita convertir el producto
cartesiano completo en una campaña inmanejable sin perder fronteras, baselines
ni casos adversariales.
