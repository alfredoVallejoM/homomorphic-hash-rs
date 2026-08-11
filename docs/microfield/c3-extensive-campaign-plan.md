# C3: campaña extensiva de validación y rendimiento de Algesum

Fecha: 2026-08-11. Estado: **preflight local y C3-C0 consolidados para las 15
suites; D2 soak y campaña multihost `Controlled` pendientes**.

La implementación y los resultados iniciales quedan fijados en
[`c3-p0-implementation-and-preflight-report.md`](c3-p0-implementation-and-preflight-report.md)
y
[`c3-f3-s3-implementation-and-preflight-report.md`](c3-f3-s3-implementation-and-preflight-report.md).
El cierre de estructuras y DB en memoria está en
[`c3-t1-r1-d1-implementation-and-preflight-report.md`](c3-t1-r1-d1-implementation-and-preflight-report.md).
El pipeline rápido, las matrices exactas y el DAG están en
[`c3-g1-g2-implementation-and-preflight-report.md`](c3-g1-g2-implementation-and-preflight-report.md).
Los corpora externos, wires y packaging están en
[`c3-x1-x2-implementation-and-preflight-report.md`](c3-x1-x2-implementation-and-preflight-report.md).
Los sistemas PostgreSQL están en
[`c3-d2-postgresql-systems-preflight-report.md`](c3-d2-postgresql-systems-preflight-report.md).
El cierre de referencias, backends y patrones F1/F2 está en
[`c3-f1-f2-closure-preflight-report.md`](c3-f1-f2-closure-preflight-report.md).
El resultado agregado y el orden exacto de los gates restantes están en
[`c3-preflight-consolidated-results-and-controlled-next.md`](c3-preflight-consolidated-results-and-controlled-next.md).
El cierre semántico está en
[`c3-c0-semantic-results.md`](c3-c0-semantic-results.md).

Este documento sustituye cualquier propuesta de ejecutar C3 sobre unas pocas
celdas «seleccionadas». Algesum reúne un núcleo de campos finitos, algoritmos
batch y packed, resúmenes y firmas algebraicas homomórficas, estructuras
persistentes, reconciliación, integración de bases de datos y análisis de
grafos. Una campaña publicable debe representar esa amplitud y declarar qué
pregunta responde cada medición.

C3 no se reducirá a un subconjunto cómodo de resultados favorables.

Las firmas y los llamados hashes homomórficos de Algesum son **resúmenes
algebraicos no criptográficos**. C3 no medirá ni sugerirá autenticación,
resistencia criptográfica a colisiones o seguridad frente a adversarios.

## 1. Objetivos y preguntas de investigación

C3 debe producir evidencia separada para:

1. corrección algebraica, equivalencia entre implementaciones y rechazo de
   entradas inválidas;
2. coste elemental de cada campo, backend y algoritmo derivado;
3. coste de construir, combinar, actualizar, retirar, serializar y restaurar
   cada familia de firma;
4. escalado con elementos, bytes, fragmentación, evaluaciones y memoria;
5. puntos de equilibrio entre actualización incremental, lote, partición y
   reconstrucción exacta;
6. comportamiento de grafos según tamaño, densidad, simetría, etiquetas,
   degeneración, tipo de incidencia y presupuesto exacto;
7. latencia, throughput, backlog y recuperación de la vertical PostgreSQL;
8. portabilidad entre ISA, fabricantes, arquitecturas, compiladores y rutas
   portable/detected;
9. coste de las garantías: validación, checks de contexto, wires, snapshots,
   límites, checkpoints y replay;
10. fronteras reales de memoria, tiempo y precisión, incluyendo resultados
    inconclusos y fallos esperados.

No habrá una cifra única de «rendimiento de Algesum». Los resultados se
publicarán por familia, operación, ruta, escala y entorno.

## 2. Tres carriles complementarios

| Carril | Unidad | Cobertura | Propósito |
|---|---|---|---|
| C3-Semantic | caso verificable | exhaustiva sobre API, leyes, límites y fallos | demostrar corrección y cerrar huecos |
| C3-Scaling | celda cronometrada | cruces completos de ejes primarios y cobertura combinatoria de secundarios | estimar coste, escala y puntos de equilibrio |
| C3-Systems | escenario sostenido | corpus, PostgreSQL, persistencia, concurrencia y fallos | demostrar representatividad operacional |

Un caso semántico puede ejecutar miles de ejemplos generados sin convertirse
en una celda de timing. Una celda de timing solo se admite si tiene checksum,
oráculo o baseline semánticamente equivalente. Un escenario de sistema mide
fases e I/O reales y nunca se mezcla con un microbenchmark.

## 3. Regla de cobertura

La fuente de verdad es `validation/rc/supported-surface-v1.json`. Toda
capacidad `supported`, `conditional`, `experimental`, `restricted` o
`legacy-adapter` debe aparecer en el ledger C3 con una de estas decisiones:

- `semantic+timed`: corrección y rendimiento publicable;
- `semantic-only`: se valida, pero cronometrarla no respondería una pregunta
  útil o incentivaría un uso incorrecto;
- `context-only`: cifra descriptiva sin comparación ni claim;
- `excluded`: solo con razón técnica, responsable y criterio de reapertura.

No se permite excluir una capacidad porque el piloto sea caro o lento. C2
sirve para calibrar duración, no para borrar familias de C3.

## 4. Diseño factorial sin falsa exhaustividad

Cruzar todos los valores de todos los ejes produciría millones de celdas y
menos repeticiones útiles. C3 aplica estas reglas reproducibles:

1. **cruce completo de ejes primarios** de cada suite: operación, familia o
   ley, implementación/baseline y escala;
2. **todos los extremos y fronteras**: cero cuando sea válido, uno, cambio de
   palabra/lane, cambio de cache, límite de partición, límite de wire y límite
   documentado;
3. **covering array de fuerza 2** para ejes secundarios: payload, distribución,
   campo secundario, alineación, fragmentación, etiquetas y patrón de edición;
4. **fuerza 3 dirigida** para interacciones con riesgo conocido, por ejemplo
   tamaño × densidad × distribución en DB y vértices × grado × simetría en
   grafos;
5. todas las celdas que definan un punto de equilibrio se conservan aunque el
   piloto muestre poca diferencia;
6. la reducción por redundancia requiere resultados equivalentes en dos
   pilotos, justificación versionada y cobertura conservada por otra celda.

El generador de manifiestos deberá publicar el conjunto de factores, la
semilla del covering array y un informe de cobertura de pares y triples. Las
celdas no se escribirán a mano como lista opaca.

## 5. Suites C3

### F1 — campos estáticos binarios

- campos: GF(2^128), las dos identidades GF(2^256) y al menos tres campos
  generados de grados pequeño, no alineado y grande;
- operaciones: add, sub, mul, square, invert, pow, reduce, canonical
  encode/decode y comparación;
- backends: referencia, portable, detected, PCLMUL/VPCLMUL y PMULL donde sean
  elegibles;
- tamaños batch: `1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256,
  4.096, 65.536, 1.048.576`;
- datos: cero, uno, máximos canónicos, sparse bits, dense bits, alternantes,
  pseudoaleatorios y operandos repetidos;
- salidas: ns/op, elementos/s, bytes/s, ciclos/op, instrucciones, branches,
  fallos de cache y energía cuando el host lo permita.

Cada backend optimizado se compara diferencialmente con el portable y el
modelo de referencia. Inversión de cero se mide como rechazo, no como dato de
latencia normal.

### F2 — campos primos

- Fp251, Goldilocks, Fp256 genérico y perfiles generados de 16, 32, 64, 127,
  192 y 256 bits cuando estén certificados;
- suma, resta, producto, cuadrado, inversión, exponenciación, reducción,
  Montgomery/radix, encode/decode y rango canónico;
- portable frente a BMI2/ADX/AVX2 u otra ruta realmente seleccionada;
- operandos en `0, 1, p-1, p/2`, con carry largo, cercanos al módulo,
  distribuciones uniformes y sesgadas;
- batches y fronteras iguales a F1, adaptadas al layout del campo.

### F3 — engines batch, packed y algoritmos derivados

- AoS scalar, batch portable, batch detected, packed owned y packed view;
- pack, unpack, add/mul/square into y assign, buffers alineados y offsets
  válidos no alineados;
- workspace nuevo frente a reutilizado;
- Horner, powers, prefix/suffix scan, batch inversion, masks y selección;
- tamaños alrededor del ancho SIMD, cache L1/L2/L3 y memoria principal;
- cold allocation, hot reuse y pipeline end-to-end por separado.

Se informará el coste de conversión. Un kernel packed no se comparará con
scalar omitiendo pack/unpack salvo que ambas cifras se presenten juntas.

### F4 — campos runtime, generación y artefactos

- binarios y primos runtime, equivalentes estáticos cuando existan;
- parse, normalización, validación/certificación, planificación, generación,
  compilación de consumidor, carga y primera operación;
- grados `8, 9, 16, 31, 32, 64, 127, 128, 192, 233, 256, 384, 512` donde la
  fábrica los admita;
- manifiestos válidos, irreducibles difíciles, certificados incompletos,
  límites excedidos, identidad incompatible y corrupción de artefactos;
- operación caliente runtime frente a estática, separando setup.

### S1 — firmas base

- aditiva, secuencia, bidireccional y multiconjunto;
- build, ingest unitario y por lote, combine/concatenate, retirada cuando la
  API conserva autoridad exacta, residual algebraico, revisión y wire;
- tamaños `0, 1, 2, 8, 64, 512, 4.096, 65.536, 1.048.576`;
- payloads `0, 1, 7, 8, 16, 64, 256, 1.024, 64 KiB, 1 MiB` donde sean
  válidos;
- datos únicos, repetidos, Zipf, cero codificado, ordenados, inversos y
  bloques con prefijo común;
- fragmentos `1, 2, 3, 8, 64, 1.024, 65.536`, equilibrados y fuertemente
  desbalanceados;
- composición izquierda, derecha, árbol balanceado y orden aleatorio válido.

Las hipótesis principales son linealidad del build en bytes y coste de
composición independiente del tamaño lógico resumido. Ambas se contrastan,
no se presuponen.

### S2 — firmas multievaluación y runtime

- multiconjunto y secuencia con `K = 1, 2, 3, 4, 8, 16` tras ampliar el
  harness; K altos son caracterización, no compromiso de API;
- build, merge/concatenate, actualización, compact wire y restauración;
- campos GF(2^128), GF(2^256), primo representativo y runtime equivalente;
- offsets/bases válidos, incompatibles, cercanos, con factores cero y casos
  que fuerzan contadores de cero;
- cruce completo `ley × K × operación × escala`; payload, campo y patrón se
  cubren mediante fuerza 2/3.

### S3 — tracked, snapshots, deltas y journals

- tracked sequence/multiset: insert, remove, multiplicidad, snapshot y restore;
- compact snapshot frente a tracked snapshot;
- delta por ley: generate, encode, validate, apply, rollback e idempotencia;
- journal: append, replay, duplicado, hueco, truncación, corrupción y límites;
- revisiones desde `0` hasta cercanas al máximo, namespaces/contextos
  compatibles e incompatibles;
- tamaños wire, bytes retenidos, asignaciones y memoria pico además de tiempo.

La corrección exige estado final idéntico a reconstrucción desde la fuente
exacta. Un residual nunca autoriza pertenencia o retirada.

### T1 — archivos, chunks, summary tree y checkpoints

- archivos de `0 B, 1 B, 4 KiB, 1 MiB, 64 MiB, 1 GiB` y preflight para
  `10 GiB`;
- chunk sizes `512 B, 4 KiB, 64 KiB, 1 MiB, 8 MiB`;
- ediciones append, prepend, middle, boundary-crossing, clustered, scattered,
  overwrite, grow y shrink;
- densidades `1 elemento, 0,01 %, 0,1 %, 1 %, 5 %, 25 %, 50 %, 75 %, 100 %`;
- sequential, coalesced bulk, adaptive y rebuild;
- checkpoint limpio, restore, versión anterior compatible, truncación,
  corrupción y límite excedido.

### R1 — reconciliación acotada

- universe `256, 4.096, 65.536` y máximo admitido;
- tamaños de conjunto desde vacío hasta 100 % del universo;
- distancia simétrica `0, 1, 2, 4, 8, 16, 32, 64` y justo por encima del
  límite configurado;
- diferencias equilibradas y unilaterales, elementos en extremos, conjuntos
  densos, sparse, clustered y pseudoaleatorios;
- sketch, merge/difference, wire, decode y verificación exacta;
- baseline: transmisión/ordenación exacta y rebuild semánticamente equivalente.

Se publicarán tasa de éxito por dominio válido, coste frente a distancia,
memoria y salida `LimitExceeded`/`Inconclusive`. No se interpretará como
reconciliación de multiconjuntos.

### D1 — base de datos en memoria

- filas `0, 1, 256, 4.096, 65.536, 1 M, 10 M`;
- esquemas narrow, medium y wide; tipos nulos/no nulos, claves secuenciales,
  UUID-like y skewed; payload `16 B–64 KiB`;
- particiones `1, 16, 64, 256, 1.024, 4.096, 16.384`;
- inserts, updates, deletes y mezclas `80/10/10`, `10/80/10`, `10/10/80`;
- densidades desde una fila hasta 100 % y distribuciones clustered, strided,
  hotspot, uniform, Zipf y particiones alternantes;
- aplicación individual, bulk, adaptive, rebuild por partición y rebuild total;
- transacción válida, duplicada, fuera de orden, abortada e incompatible;
- MFTL replay, checkpoint, restore, redelivery y reconstrucción autoritativa.

Cada celda verifica filas, revisiones, particiones y resumen global contra un
modelo exacto. Se medirá generación, validación, agrupación, aplicación,
publicación y verificación por separado.

### D2 — PostgreSQL y change stream

- PostgreSQL en al menos dos versiones soportadas, almacenamiento NVMe local y
  parámetros versionados;
- datasets sintéticos controlados de 65.536, 1 M, 10 M y 100 M filas tras
  preflight, más un fixture relacional público con checksum y licencia;
- 1, 2, 8, 16, 32, 64, 128 y 256 writers/readers según capacidad del host;
- transacciones de `1, 8, 64, 256, 4.096, 65.536` mutaciones;
- entrega directa y logical decoding; batching por tiempo y tamaño;
- latencia de commit observada por cliente, captura, aplicación y
  disponibilidad del resumen; throughput y backlog sostenidos;
- steady state de 15 min, 1 h y soak de 8 h para configuraciones principales;
- crash antes/después de commit/checkpoint, kill del consumidor, restart de
  PostgreSQL, redelivery, slot retenido, truncación permitida y rebuild;
- migración compatible/incompatible de schema y rebootstrap;
- límites de backpressure y recuperación tras ráfagas 2×/5×.

Puertas: cero commits perdidos o dobles, LSN/posición monótonos, estado exacto
tras restart, resumen igual a rebuild y backlog recuperado. p95/p99 no se
derivan de microbenchmarks.

### G1 — filtro rápido, invariantes y actualización incremental

Familias sintéticas:

- path, cycle, star, complete, complete bipartite, tree balanceado y skewed;
- grid 2D/3D, torus, regular, Erdős–Rényi, Barabási–Albert y small-world;
- DAG chain/fan-in/fan-out/layered, desconectados, multiaristas, lazos e
  hiperaristas/incidencias cuando el modelo los admita.

Ejes:

- vértices `0, 1, 2, 8, 16, 64, 256, 4.096, 16.384, 131.072, 1 M`;
- grado medio `0, 1, 2, 4, 8, 16, 32, 64, 128, sqrt(V), V-1` donde aplique;
- etiquetas homogéneas, únicas, cardinalidad 2/16/256, Zipf y correlacionadas
  con grado/componente;
- orden de entrada canónico, inverso, BFS, DFS y permutado por semilla;
- build/model, prepare, canales individuales, pipeline fast, global,
  comparación pareada e incremental;
- cambios de etiqueta, arista, vértice, lote clustered/scattered y componente;
- rondas, vértices auditados, escalada, memoria, paralelismo `1, 2, 4, 8, 16,
  32` threads y NUMA cuando exista.

Toda comparación de isomorfismo usa pares relabelados y pares cercanos no
isomorfos. La firma rápida solo produce evidencia negativa o
`Indistinguishable`, nunca una prueba de igualdad.

### G2 — canonización exacta, degeneración y DAG canónico

- familias asimétricas y simétricas: paths, cycles, strongly regular, cages,
  unions, twins, regular covers y corpus adversariales versionados;
- tamaños exactos `4, 6, 8, 10, 12, 14, 16, 20, 24, 32, 48, 64` según
  dificultad, sin fingir que todos terminarán;
- presupuestos de nodos `1, 10, 100, 1k, 10k, 100k, 1M`, profundidad,
  memoria y deadline;
- rutas discrete refinement, individualization/refinement, paired comparison,
  compact encoding, mapping verification y long walks;
- resultado exacto o inconcluso, límite agotado, nodos explorados, hojas,
  profundidad, órbitas/particiones y certificado/mapping verificado;
- DAG: miss, insert exacto, reuse, actualización, persistencia, restore,
  corrupción y prohibición de insertar resultados inconclusos.

Se ejecutarán oráculos independientes sobre todos los grafos pequeños y
muestreo estratificado de los mayores. Los timeouts e inconclusos forman parte
del resultado y nunca se descartan del denominador.

### X1 — corpus externos y verticales representativas

- Graph Atlas completo dentro de su rango, MUTAG, Email-EU, Diseasome y nuevos
  corpus públicos de redes, moléculas y DAG con licencia/checksum fijados;
- estratificación por tamaño, densidad, componentes, etiquetas y simetría;
- todos los pares isomorfos generados por relabeling y una muestra registrada
  de pares negativos cercanos;
- cargas de química, lógica DIMACS y redes ya presentes en el repositorio;
- ingest, parse, mapping, preparación, análisis, actualización y salida por
  fases;
- datasets sintéticos como control y externos como representatividad, nunca
  promediados en una sola cifra.

### X2 — wires, compatibilidad, packaging y herramientas

- todas las familias MFSG/MFTS/MFDE/MFDJ/MFFC/MFST/MFRS/MFRW/MFTX/MFTL/MFGD;
- round-trip por versión, golden vectors, bytes mínimos/máximos, truncación en
  cada frontera estructural, trailing bytes, longitudes falsas y límites;
- compatibilidad entre feature sets, consumidor externo y MSRV/current Rust;
- parse/plan/generate/verify/publish de artefactos en cold y warm filesystem;
- package limpio, examples y binarios sobre x86-64 y AArch64.

## 6. Hosts y replicación

Las cifras principales requieren, como mínimo, cuatro clases de host dedicado:

| Clase | Propósito |
|---|---|
| x86-64 Intel moderna | PCLMUL/VPCLMUL/BMI2 y continuidad con calibración actual |
| x86-64 AMD moderna | separar ISA de microarquitectura/fabricante |
| AArch64 servidor | PMULL y comportamiento Linux ARM |
| x86-64 portable-forced | baseline sin selección ISA en el mismo hardware |

Deseable, como réplica secundaria: otra generación Intel, otra generación AMD
y AArch64 de escritorio. No se fusionan hosts heterogéneos en una mediana.
Cada claim debe reproducirse en dos hosts de la misma clase o quedar marcado
como resultado de un solo sistema.

Por host se registran CPU, microcode, caches, NUMA, RAM, kernel, filesystem,
Rust/LLVM, flags, backend, governor, turbo, afinidad, temperatura, energía,
swap, mitigaciones y procesos residentes. Las suites I/O usan dispositivo y
filesystem dedicados y registran estado SMART/temperatura cuando sea posible.

## 7. Protocolo temporal y estadístico

- mínimo 30 y máximo 100 procesos independientes por celda;
- objetivo principal: semianchura relativa CI95 <= 3 % para mediana en celdas
  de claims, <= 5 % en caracterización y tails;
- 50–200 observaciones internas calibradas por proceso para microbenchmarks;
- bloques por host y día; al menos tres días para suites principales;
- orden de celdas aleatorio determinista, A/B alternado y seeds emparejadas;
- mediana, p5/p25/p75/p95/p99, MAD, CI95, throughput y tamaño de efecto
  pareado; tails mediante bootstrap jerárquico;
- slopes solo con cinco escalas válidas y diagnóstico de cambio de régimen;
- outliers retenidos y marcados; fallos, OOM, timeout e inconclusos retenidos;
- corrección por multiplicidad o control de FDR para familias de comparaciones;
- significancia nunca sustituye tamaño de efecto ni relevancia práctica.

Las cargas sostenidas usan ventanas temporales y repeticiones de escenario,
no millones de observaciones correlacionadas tratadas como independientes.

## 8. Métricas obligatorias

Además de duración:

- operaciones, elementos y bytes por segundo;
- asignaciones, bytes asignados y RSS/pico;
- tamaño wire/snapshot/checkpoint y amplification;
- ciclos, instrucciones, IPC, branches, cache/TLB y page faults cuando `perf`
  esté disponible sin multiplexado excesivo;
- energía por operación en hosts con contador fiable;
- threads, speedup, eficiencia y saturación de ancho de banda;
- DB: TPS, commit latency, capture/apply lag, backlog, fsync/WAL bytes y tiempo
  de recuperación;
- grafos: ns/vértice, ns/incidencia/ronda, estados exactos, escalada, audit set
  y reuse del DAG;
- tasa de éxito exacta, rechazo esperado, inconclusos y límites agotados.

## 9. Baselines

Baselines internos obligatorios:

- referencia/portable/detected/ISA;
- scalar/batch/packed incluyendo y excluyendo conversiones, ambas publicadas;
- estático/runtime/generado equivalente;
- build/composición y rebuild/actualización incremental;
- secuencial/bulk/adaptive/partición/rebuild total;
- proceso frío/caliente y workspace nuevo/reutilizado;
- in-memory/persistido y directo/logical decoding.

Baselines externos solo se añaden si existe equivalencia de operación,
representación, validación y seguridad. SHA-256 u otro hash criptográfico puede
ser contexto de coste de digest completo, nunca competidor semántico ni base
para decir que Algesum ofrece seguridad criptográfica.

## 10. Volumen previsto y presupuesto

El inventario inicial apunta a:

- al menos 12.000 casos C3-Semantic, muchos con cientos/miles de ejemplos;
- entre 2.500 y 4.000 celdas C3-Scaling antes de calibración;
- 150–300 escenarios C3-Systems, incluidos fallos y soak;
- mínimo teórico de 75.000–120.000 procesos de timing si todas las celdas
  convergen en 30 procesos, más réplicas por host;
- almacenamiento bruto estimado de 50–250 GiB según counters y trazas;
- ejecución por shards reanudables; nunca una campaña monolítica.

Antes de prometer calendario, C3-P0 medirá duración y tamaño de 1 % de cada
suite. Ese preflight estima host-días, energía y almacenamiento. Si el coste
obliga a reducir, se reduce repetición adaptativa en celdas ya precisas o se
aplica covering array documentado; no se elimina una familia ni una frontera.

## 11. Shards y orden de ejecución

1. `C3-P0`: ampliar harness, ledger, generador factorial y preflight del 1 %;
2. `C3-C0`: semantic exhaustivo, fuzz/adversarial y oráculos;
3. `C3-F`: F1–F4 campos, engines y generación;
4. `C3-S`: S1–S3 firmas, snapshots y deltas;
5. `C3-T/R`: árboles, archivos y reconciliación;
6. `C3-D1`: base de datos en memoria;
7. `C3-G1`: grafos rápidos, globales e incrementales;
8. `C3-G2`: exacto, degeneración y DAG;
9. `C3-X`: corpus, wires, herramientas y packaging;
10. `C3-D2`: PostgreSQL, concurrencia, WAL, recovery y soak;
11. `C3-R`: réplica multihost de celdas de claims y casos frontera;
12. `C3-P`: análisis, revisión adversarial, gráficos y paquete publicable.

Cada shard tiene manifest, raw, ambiente, checksums, informe y commit propios.
Un shard verde no autoriza claims de otro.

## 12. Gates

### Gate de entrada

- inventario API/operaciones cerrado y ledger sin capacidades huérfanas;
- workloads faltantes implementados con oráculo;
- manifests generados y schemas validados;
- preflight de memoria/tiempo/almacenamiento aprobado;
- hosts y configuración identificados.

### Gate por celda

- checksum/oráculo correcto en todas las réplicas;
- baseline semánticamente equivalente identificado;
- entorno `Controlled`, sin swap ni throttling invalidante;
- precisión alcanzada o resultado `Inconclusive` explícito;
- raw completo, no sobrescrito y ligado a commit/binario.

### Gate de publicación

- cobertura del 100 % del ledger;
- cero discrepancias semánticas abiertas;
- réplica de claims principales en x86-64 Intel, x86-64 AMD y AArch64;
- limitations y resultados negativos incluidos;
- tablas y figuras regenerables desde raw;
- revisión independiente de metodología y claims;
- lenguaje explícito de resumen algebraico no criptográfico.

## 13. Trabajo de implementación previo

El harness actual cubre una fracción útil, pero insuficiente, de esta matriz.
Antes de ejecutar C3 deben añadirse:

- operaciones field add/sub/square/invert/pow/encode/decode, prime/binary
  batch/packed y algoritmos derivados;
- campos runtime y generación por grados/perfiles;
- firmas tracked, remove/trim, wire/restore, K configurables y campos variados;
- delta por ley, journal, chunks, snapshots/checkpoints y métricas de memoria;
- reconciliación sketch/merge/decode/fallos por separado;
- DB por mezcla de mutación, schema/payload, replay/checkpoint y telemetría de
  ruta elegida;
- generadores completos de grafos, canales individuales, paralelismo,
  incremental por lotes y métricas exactas/DAG;
- worker de escenarios sostenidos, fault injection y PostgreSQL logical
  decoding;
- soporte de counters, covering arrays, shards, resume y estimador de coste.

C3 no comienza hasta que esta deuda del harness tenga pruebas diferenciales y
smoke en CI.

## 14. Artefactos de salida

Cada resultado deberá permitir reconstruir exactamente:

- commit, binario y feature set;
- dataset/corpus y checksum;
- manifest expandido y factores originales;
- host/configuración/día/orden/seed;
- observaciones, contadores, errores e inconclusos;
- agregados y comparaciones generados, no editados;
- cobertura alcanzada y huecos;
- coste total de ejecución;
- claims permitidos, condicionados y prohibidos.

La campaña solo se considerará cerrada cuando otra máquina pueda regenerar
los informes desde los artefactos versionados y cuando el ledger muestre la
totalidad de la superficie admitida.
