# F6.G15 — preparación interna de firmas, campos y grafos

Fecha de planificación: 4 de agosto de 2026.

Estado: planificado. G15 no es una fase de grafos: convierte la biblioteca
completa —campos finitos, firmas homomórficas, protocolos derivados y motor de
grafos— en una capacidad interna gobernada, reproducible y operable.

Las firmas son un producto de primer nivel. El motor de grafos es una aplicación
importante de esas primitivas y de `Microcanon`, pero no define por sí solo el
alcance ni la API de la biblioteca.

La auditoría específica de deltas para archivos, bases de datos y árboles
jerárquicos está en
[`phase-6-signature-delta-audit.md`](phase-6-signature-delta-audit.md).

## 1. Decisión ejecutiva actual

La biblioteca es hoy un **release candidate interno condicionado**:

| Uso | Estado actual | Condición |
|---|---|---|
| Campos estáticos/generados | Apto | identidad y assurance obligatorios |
| Firmas aditiva, secuencia y multiset | Apto como primitivas | igualdad significa `Indistinguishable` |
| Bidireccional y multievaluación | Candidato | congelar perfiles y coste/beneficio |
| `TrackedSequence`/`TrackedMultiset` | Apto | exactitud a costa de memoria O(n) |
| Residuales algebraicos | Rechazado como prueba | exponer solo como ecuación algebraica |
| Reconciliación acotada | Validada pero no productizada | mover el decoder desde validation-lab |
| Campos runtime | Apto condicionado | conservar assurance y límites explícitos |
| Prefiltro de grafos | Apto | usar identidad completa de perfil |
| Comparación exacta puntual | Apto | manejar siempre `Inconclusive` |
| Clave canónica para deduplicación | Candidato | congelar schema, envelope y política de persistencia |
| Actualización local de labels | Apto | estado versionado y fallback habilitado |
| Edición topológica incremental | Correcta, no optimizada | no prometer mejora hasta medir CSR end-to-end |
| Química/redes/hipergrafos | Experimental | cerrar adapters y equivalencia de dominio |
| API pública o servicio con SLA | No apto todavía | queda fuera de G15 interno |

Por tanto, ya puede emplearse en experimentos internos y herramientas
controladas. G15 será el gate para depender de ella tanto en agregación,
streaming, reconciliación e índices compactos como en deduplicación de cliques y
subredes convertidas en DAG.

## 2. Separación entre cierre interno y publicación

G15-I cubre:

- API soportada de firmas y campos;
- protocolos completos de composición, streaming y reconciliación;
- serialización, persistencia e interoperabilidad de firmas;
- superficie soportada y perfiles de ejecución;
- schemas de aplicación y persistencia exacta;
- adapters internos con pérdida semántica imposible o detectable;
- campañas diferenciales, fuzzing y límites;
- presupuestos, observabilidad y runbook;
- un artefacto reproducible de decisión go/no-go.

Quedan para una futura G15-P/publicación:

- estabilidad semver 1.0 y compatibilidad de largo plazo;
- licencia definitiva, crates.io, supply-chain y documentación comercial;
- claims de rendimiento multi-microarquitectura;
- equivalencia química o científica general;
- SLA externo, soporte de formatos arbitrarios y migraciones automáticas.

## 3. Contrato de consumo interno

### 3.1 Contrato de las firmas homomórficas

Cada firma expuesta debe declarar conjuntamente:

```text
SignatureLaw
SignatureAssurance
FieldId / DynFieldId
EncoderId
SignatureId
parámetros algebraicos
wire schema
contadores y límites
```

La API soportada se dividirá por semántica, no por conveniencia:

| Familia | Ley ofrecida | Uso interno permitido |
|---|---|---|
| `AdditiveSignature` | suma/partición | checksum algebraico, merge y delta |
| `SequenceSignature` | concatenación Horner | chunks, logs y streams ordenados |
| `BidirectionalSequenceSignature` | dos orientaciones | paths y trazas reversibles |
| `MultisetSignature` | producto con multiplicidad | inventarios y candidate index |
| `MultiEvaluationMultisetSignature` | producto en K puntos | reducir candidatos, no probar igualdad |
| `MultiEvaluationSequenceSignature` | secuencia en K bases | exactitud acotada sobre elementos codificados |
| `Tracked*` | contenido retenido exacto | confirmación interna con memoria O(n) |
| `AlgebraicResidual` | recomposición de ecuación | diagnóstico; nunca membership proof |

Las variantes runtime deberán tener la misma ley y wire que su presentación
estática equivalente. No se expondrá un alias “hash” que oculte la ley,
assurance, campo o encoder.

No se atribuye a estas firmas seguridad, autenticidad, resistencia adversarial
ni valor criptográfico. Los digests empleados para identificar descriptores o
artefactos son infraestructura de versionado y no forman parte de la garantía
algebraica.

### 3.2 Tres niveles admitidos para grafos

1. **Candidate filtering**: metadatos, grados, firmas y patrones pueden descartar.
   La igualdad solo conserva el par en el mismo bucket.
2. **Exact comparison**: `AdaptiveGraphPipeline` debe alcanzar `Exact` y devolver
   `Isomorphic(mapping)` o `Different`. `Inconclusive` se propaga al llamador.
3. **Canonical persistence**: solo una `CanonicalGraphForm` exacta y completa
   puede originar `CanonicalGraphKey` y un nodo del DAG persistente.

No se permite usar como clave definitiva `FastGraphSignature`, SHA híbrido,
`GraphDiscriminationDigest`, perfiles de patterns ni 2-WL localizado.

### 3.3 Persistencia de firmas

El wire `MFSG` autocontenido será el formato soportado para firmas compactas.
La persistencia deberá almacenar sus bytes completos y validar al restaurar
schema, ley, campo, encoder, parámetros y contadores. `Tracked*` requiere un
envelope distinto porque el wire compacto no contiene los elementos exactos.

Una firma nunca se comparará usando únicamente sus lanes de campo. Antes de
combinar, reconciliar o comparar se exige compatibilidad de `SignatureId` y
contexto completo. Los cambios incompatibles de wire o parámetros crean un
namespace nuevo.

### 3.4 Identidad canónica persistida de grafos

Toda entrada canónica interna conservará, como mínimo:

```text
schema_version
GraphSchemaId
CanonicalGraphEncodingId
CanonicalGraphKey
canonical_bytes
producer_version
```

El perfil rápido y el campo pueden indexar candidatos, pero no forman parte de
la identidad matemática exacta. Cambiar schema o encoding obliga a un namespace
nuevo; nunca se reinterpretan claves antiguas silenciosamente.

### 3.5 Semántica de errores

- error de firma: estado y contador anteriores intactos;
- contexto incompatible: no comparar ni convertir silenciosamente;
- reconciliación fuera de cota: resultado tipado sin candidato parcial;
- `Different`: witness negativo utilizable.
- `Isomorphic`: mapping verificado utilizable.
- `Inconclusive`: elemento pendiente, no diferente ni isomorfo.
- error de delta: estado anterior intacto.
- revisión obsoleta: relectura y reintento explícito, nunca overwrite.

Las verificaciones se clasificarán como `AlgebraicConsistency`,
`SourceAuthorized` o `ExactRebuild`. Comprobar una recomposición de campo nunca
se presentará como prueba de pertenencia, integridad o autenticidad.

### 3.6 Arquitectura de producto

La dirección de dependencias y de producto será:

```text
campos + encoders
        │
        ▼
firmas homomórficas ──► deltas / streaming / reconciliación / índices
        │
        └─────────────► firmas estructurales de grafos
                                  │
                                  ▼
                      filtro ─► Microcanon ─► DAG exacto
```

Los campos y las firmas constituyen la plataforma reutilizable. Grafos,
cliques, moléculas y redes son consumidores de esa plataforma y añaden una
autoridad exacta propia; no estrechan la API general ni convierten una firma
probabilística en prueba de igualdad.

## 4. Hitos

### G15.0 — inventario y superficie soportada

Entregables:

- allowlist de tipos y métodos internos soportados;
- clasificación `Supported`, `Experimental`, `LegacyAdapter` o `Rejected`;
- inventario separado de campos, encoders, firmas, protocolos y grafos;
- tabla de leyes, assurance, coste, composabilidad y pérdida de información;
- eliminación de ejemplos que comparen fingerprints como igualdad;
- auditoría de defaults peligrosos: un schema persistente no usará
  `GraphSchemaId::default()` accidentalmente;
- guía de selección entre campo estático generado, F251 y campo runtime;
- guía `SignatureFieldProfile` por característica, cardinalidad, ley, K,
  representación y backend disponible;

Gate:

- todo ejemplo interno compila usando únicamente la allowlist;
- ninguna API legacy aparece en el flujo recomendado;
- claims y assurance son coherentes en Rustdoc, README e informes.

### G15.1 — API soportada de firmas homomórficas

Entregables:

- constructores y builders coherentes para las seis familias mantenidas;
- nombres explícitos `combine`, `concatenate`, `insert`, `remove`, `rollback` y
  `difference_residual` según la ley real;
- paridad estática/runtime y puente de campos externos generados;
- selección documentada de encoder canónico, binario, primo y hash-to-field
  multicanal, sin convertir el mixer en una garantía de la firma;
- profiles recomendados para una evaluación y multievaluación K=2/K=4;
- wire round-trip, límites, contadores y errores homogéneos;
- deprecación de aliases legacy que oculten ley o identidad;
- ejemplos completos de agregación, secuencia, multiset y tracking.

Gate:

- todas las firmas pasan la misma matriz de leyes sobre campos binarios, primos,
  generados y runtime compatibles;
- static/dynamic producen el mismo wire para la misma presentación;
- cada error representable deja estado y salida intactos;
- ejemplos y compile-fail impiden combinar identidades incompatibles;
- el residual no puede consumirse desde una API llamada `verify_membership`.

### G15.2 — protocolos y persistencia de firmas

Entregables:

- envelope soportado para firmas compactas y snapshots rastreados;
- agregación distribuida de particiones con negociación de `SignatureId`;
- checkpoints y composición de streams sin releer el prefijo;
- promoción del decoder de reconciliación desde validation-lab a módulo
  soportado, con universo/degree/memory bounds explícitos;
- reconciliación de multiconjuntos o declaración expresa de que v1 acepta solo
  conjuntos sin multiplicidad;
- APIs batch para ingestión, combinación y restauración transaccionales;
- namespaces y política de migración de wire/field/encoder;
- envelope común de delta con contexto, namespace y revisiones source/target;
- `AdditiveDelta` y `MultisetDelta` con secciones removed/added;
- `SequenceAppend`, `SequenceTrim` y edición por camino jerárquico;
- aplicación de deltas en dos fases: preflight completo y publicación atómica;
- distinción tipada entre consistencia algebraica, autorización de la fuente y
  reconstrucción exacta.

Gate:

- particionar y recomponer coincide con ingestión directa en todos los árboles
  normativos;
- snapshots sobreviven restart y rechazan corrupción/context drift;
- reconciliación recupera toda diferencia dentro de cota y falla cerrada fuera;
- ninguna ausencia de colisión empírica se convierte en claim de exactitud;
- tracking exacto y firma compacta permanecen tipos semánticamente distintos;
- replay, revisión obsoleta y retirada no autorizada fallan sin mutar estado;
- un delta aplicado coincide con reconstrucción completa tras cada operación.

### G15.3 — aplicaciones internas de las firmas

Verticales obligatorios:

1. agregación distribuida de inventario/telemetría;
2. archivos mediante secuencias de chunks y edits versionados;
3. bases de datos mediante filas canónicas y before/after images;
4. árbol jerárquico de firmas compatible con una topología Merkle;
5. reconciliación acotada de conjuntos;
6. índice compacto multi-canal con confirmación exacta externa;
7. delta y rollback mediante fuente autoritativa o tracking donde corresponda.

Cada vertical se contrastará con reconstrucción y almacenamiento exactos y,
cuando aporte información, con otro resumen algebraico no criptográfico
mantenido. Medirá CPU, I/O, memoria, bytes comunicados, reducción de candidatos
y coste de confirmación. Un vertical solo se promueve si mejora una dimensión
relevante sin debilitar la corrección.

Gate:

- al menos archivos, base de datos, árbol, agregación y reconciliación alcanzan
  `ValidatedApplication` o permanecen explícitamente `ValidatedPrimitive`;
- se publican colisiones y puntos de degeneración por perfil;
- el coste de construir firmas nunca se oculta al comparar un merge barato;
- el caller debe elegir cuándo y cómo realiza confirmación exacta.
- el árbol recompone una edición de hoja en O(log n) para su forma congelada;
- ningún canal algebraico se documenta como raíz autenticada.

### G15.4 — grafos, DAG y adapters de dominio

Entregables de infraestructura:

- perfiles `FastFilter`, `Balanced` y `ExactBounded` con budgets;
- fachada que no convierta `Inconclusive` en bool;
- envelope versionado para nodos canónicos;
- índice de candidatos separado de identidad exacta;
- algoritmo `lookup → filter → exact → insert/reuse` transaccional;
- aristas DAG referenciando `CanonicalGraphKey`, nunca firmas rápidas.

Orden de implementación:

1. adapter genérico para cliques y subredes con frontera etiquetada;
2. red dirigida/knowledge graph;
3. hipergrafo relacional;
4. química, solo si se declara exactamente qué equivalencia representa.

Cada adapter tendrá:

- `GraphSchemaId` propio y versionado;
- tabla campo-de-dominio → vertex/relation/role/multiplicity;
- validación de pérdida antes de construir el grafo;
- fixtures de ida/vuelta y perturbación semántica mínima;
- documento de lo que deliberadamente no representa.

Para cliques, la frontera externa debe formar parte del schema si dos cliques
idénticos internamente pero conectados de forma distinta no son intercambiables.

Gate:

- carga, dirección, role, multiplicidad, loops y labels relevantes sobreviven;
- una perturbación de cada atributo cambia la forma exacta cuando el schema la
  considera significativa;
- renumerar IDs de almacenamiento no cambia la forma;
- ningún adapter deduce equivalencia científica que no esté declarada.

Además:

- dos renumeraciones deben converger en el mismo nodo y mapping;
- una colisión rápida debe terminar en nodos exactos distintos;
- `Inconclusive` no crea ni fusiona nodos;
- inserciones concurrentes equivalentes convergen en una identidad.

### G15.5 — cierre de oráculos y corpus

Entregables:

- campañas exhaustivas de leyes y colisiones para cada firma/profile mantenido;
- baselines aplicados de agregación, streaming y reconciliación;
- verificación static/dynamic y entre campos con la misma presentación;
- corpus reales versionados para los verticales de firmas elegidos;
- canonización exacta de los 12.346 representantes n=8 con 12.346 claves;
- resolución exacta de todos los buckets residuales del pipeline, no una muestra;
- muestreo n=9 y adversariales crecientes: CFI, SRG, cospectrales, grids,
  hypercubes, cliques, bicliques y complementos;
- generador exhaustivo relacional pequeño para dirección, labels, roles,
  loops, multiplicidad e hiperaristas;
- conversión gadget inyectiva a grafo simple coloreado y comparación contra
  SageMath/nauty-Traces;
- corpus externos con URL, licencia, SHA-256, parser y segundo pase offline.

Gate:

- cero discrepancias de ley, wire o restauración en firmas compatibles;
- toda colisión observada conserva el par mínimo reproducible;
- reconciliación coincide con el conjunto exacto dentro de la cota declarada;
- cero discrepancias entre compacto, referencia y oráculo admitido;
- cero falsos `Isomorphic`; todo mapping se verifica independientemente;
- presupuesto insuficiente produce exclusivamente `Inconclusive`;
- todos los manifests regeneran con diff vacío.

### G15.6 — concurrencia, fuzzing y robustez

Entregables:

- property tests de secuencias aleatorias `GraphDelta` frente a rebuild;
- property tests de operaciones aleatorias de cada firma frente a un modelo
  exacto de referencia;
- property tests de deltas de archivo, filas y hojas frente a rebuild tras cada
  paso;
- fuzz targets de wires `MFSG`, identidades y restauración static/dynamic;
- cobertura de transacciones mixtas, conflictos, overflows y revision races;
- fuzz targets para builder, encoding canónico, parser, verifier, pipeline y
  delta;
- límites para número de vértices, incidencias, tamaño de labels, comandos y
  memoria retenida;
- panic audit de toda entrada externa;
- Miri para rutas portables y ASan donde exista `unsafe` ISA/storage.

Gate:

- ningún error muta firma, contador, salida, estado de grafo o DAG;
- cada bug encontrado queda como fixture mínimo;
- fuzzing nocturno cumple la ventana acordada sin crash ni divergencia;
- las carreras optimistas producen éxito único o revision mismatch tipado.

### G15.7 — rendimiento y planificación de capacidad

Escenarios obligatorios:

- ingestión, merge, concatenación, insert/remove y restore de cada firma;
- apply/replay/rollback de deltas de archivo, base de datos y árbol;
- perfiles K=1/K=2/K=4 y comparación static/dynamic;
- reconciliación por tamaño de universo y diferencia;
- tracking exacto frente a firma compacta;
- rechazo por metadata, degree, field, patterns y pares;
- positivo y negativo exactos;
- cliques repetidos y subredes con buckets de distinta cardinalidad;
- delta de label, multiplicidad, arista y ráfaga;
- grafos sparse, densos, regulares y altamente simétricos.

Métricas:

- p50/p95/p99 y throughput;
- elementos/s, bytes comunicados y reducción de candidatos para firmas;
- memoria de tracking y coste de confirmación exacta;
- distribución de tier terminal;
- tasa de `Inconclusive`, skip y fallback;
- nodos exactos, profundidad y peak bytes;
- allocations y bytes persistidos por nodo DAG;
- hit rate de candidatos y ahorro frente a procesar duplicados de nuevo.

Gate:

- se fijan SLO internos por workload, no universales;
- ninguna familia de firmas regresa más de 5 % en su perfil congelado sin una
  decisión explícita;
- el prefiltro no regresa más de 5 % sobre su baseline congelado;
- label delta mantiene una mejora end-to-end medible;
- las ediciones topológicas se promueven como incrementales solo si superan el
  rebuild; de lo contrario se enrutan directamente al fallback;
- toda ruta respeta presupuestos incluso al devolver `Inconclusive`.

### G15.8 — runbook y release candidate interno

Entregables:

- guía de integración con ejemplos signature/merge/stream/reconcile y
  filter/exact/DAG/delta;
- tabla de perfiles recomendados y anti-patrones;
- observabilidad mínima y diagnóstico de buckets difíciles;
- procedimiento de backup, regeneración y migración de namespaces;
- SBOM/inventario de dependencias para despliegue interno, sin convertirlo aún
  en trabajo de publicación;
- CI `g15-internal` con evidencia semántica y performance host-specific.

Gate:

- un consumidor fixture externo compila sin APIs privadas;
- el consumidor ejerce al menos cuatro leyes, tracking, wire y un campo externo;
- una prueba end-to-end crea, reinicia y consulta un DAG canónico;
- el runbook resuelve deliberadamente collision, timeout, revision mismatch y
  schema drift;
- resultados deterministas coinciden en x86-64 y AArch64.

### G15.9 — auditoría go/no-go

Se generará `validation/f6/results/g15-internal-readiness-v1.json` con:

- commit y toolchains;
- manifests y digests de corpus;
- gates ejecutados y excepciones;
- matriz de capacidades `ready/conditional/not-ready`;
- clasificación individual de cada firma y protocolo;
- hashes de resultados G11–G15;
- presupuestos y SLO seleccionados;
- lista cerrada de limitaciones aceptadas.

La decisión solo puede ser:

- `ReadyForInternalUse`;
- `Conditional` con restricciones ejecutables;
- `NotReady`, indicando el gate exacto.

No se aceptará un “aprobado” narrativo sin artefacto reproducible.

## 5. Matriz de pruebas mínima

| Familia | Corrección | Metamorfismo | Adversarial | Presupuesto |
|---|---|---|---|---|
| Campos/encoders | leyes, identidad y encoding | static/dynamic/generado | input no canónico y contexto cruzado | degree/memoria |
| Firmas | leyes y wires | partición/merge | colisiones congeladas | overflow |
| Deltas de datos | estado incremental vs rebuild | replay/rollback/asociación | revisión obsoleta y retirada ausente | journal/I/O |
| Reconciliación | recuperación exacta en cota | partición/unión | fuera de cota y multiplicidad | degree/memoria |
| Pipeline | vs exacto | renumeración | CFI/SRG/cospectral | skips/ceilings |
| Microcanon | compacto vs referencia | campo/backend/perfil | alta simetría | nodes/bytes/time |
| DAG | clave/bytes/round-trip | orden/concurrencia | collision bucket | transacción |
| Delta | vs rebuild | secuencia/reordenación válida | merge/split/loops | cone/fallback |
| Adapters | ida/vuelta | renumeración externa | pérdida de atributo | tamaño de entrada |

## 6. Trazabilidad del estado actual

Ya satisfecho:

- cinco firmas segregadas más secuencia multievaluada y variantes dinámicas;
- leyes de partición/concatenación, identidades, wire y atomicidad;
- 145.636 ecuaciones metamórficas y colisiones mínimas congeladas;
- tracking exacto, factores cero y compatibilidad static/dynamic;
- reconciliación acotada validada en 63.232 pares dentro de validation-lab;
- campos mantenidos, externos generados y runtime con assurance;
- autoridad exacta independiente del campo;
- mappings verificados y resultado fail-closed;
- exhaustivo de 32.768 grafos n=6 y 156 clases;
- corpus nauty n=8 autenticado;
- CFI, Shrikhande/rook y C6/2C3;
- pipeline G13, delta G14 y campaña reproducible;
- Clippy, Rustdoc, x86/AArch64 CI y gates Microfield.

Parcial o pendiente para aprobar consumo persistente:

- decidir la allowlist definitiva de firmas y profiles K;
- productizar reconciliación y declarar el soporte de multiplicidad;
- validar aplicaciones y baselines de firmas fuera del dominio pequeño;
- persistencia soportada de snapshots compactos y rastreados;
- consumidor externo centrado en firmas, no solo tests internos;
- canonizar todos los 12.346 representantes y buckets residuales en un gate
  específico de clave exacta;
- oráculo independiente del modelo relacional completo;
- adapter de cliques/subredes y contrato de frontera;
- envelope y prueba transaccional del DAG;
- fuzzing de GraphDelta/pipeline/parser;
- SLO y budgets del workload real;
- prueba end-to-end de consumidor interno y runbook.

## 7. Orden recomendado

```text
G15.0 contrato
   ↓
G15.1 API firmas ──► G15.2 protocolos ──► G15.3 aplicaciones
   │                                            │
   └──────────────► G15.4 grafos/DAG ◄──────────┘
                          ↓
                    G15.5 oráculos
                          ↓
                    G15.6 robustez
                          ↓
                    G15.7 capacidad
                          ↓
                    G15.8 RC interno
                          ↓
                    G15.9 go/no-go
```

G15.0–G15.4 forman los verticales funcionales de firmas y grafos. G15.5–G15.7
impiden aprobarlos por intuición. G15.8–G15.9 convierten la evidencia en una
dependencia interna operable. Solo después se discutirá una fase de publicación.

## 8. Definition of Done interna

Bloquean `ReadyForInternalUse`:

1. allowlist, assurance y profiles de todas las firmas soportadas;
2. wire/snapshot, agregación, streaming y reconciliación fail-closed;
3. al menos tres aplicaciones de firmas con baseline y límites cuantificados;
4. flujo DAG transaccional basado exclusivamente en clave exacta;
5. adapter de cliques/subredes y schema de frontera cerrado;
6. n=8 completo, buckets residuales y diferencial relacional sin discrepancias;
7. property/fuzz gates de firmas, parser, verifier, pipeline y delta;
8. SLO y budgets medidos sobre los workloads internos objetivo;
9. consumidor end-to-end, CI, runbook y artefacto go/no-go reproducible.

No bloquean el consumo interno inicial:

- adapter químico general;
- benchmarks de todas las familias de CPU;
- estabilización semver o compatibilidad pública;
- claims competitivos frente a nauty/Traces;
- licencia, empaquetado y publicación.

Así G15 no se convierte inadvertidamente en una fase de producto público. Su
salida es una dependencia interna segura para el workload declarado, no una
promesa universal.
