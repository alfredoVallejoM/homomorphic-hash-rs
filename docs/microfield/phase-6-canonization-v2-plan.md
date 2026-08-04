# Plan F6.G8–G15 — Microcanon v1 y firmas estructurales reforzadas

Fecha: 3 de agosto de 2026.

Estado: F6.G8–G14 completados localmente; F6.G15 queda planificado como cierre
de consumo interno, separado de una futura fase de publicación.

La entrega G8/G9 se documenta en
[`phase-6-g8-g9-implementation-report.md`](phase-6-g8-g9-implementation-report.md).
El gate exhaustivo de 3 de agosto de 2026 recorrió los 32.768 grafos simples
etiquetados de orden seis y reprodujo exactamente las 156 clases del oráculo
factorial independiente tanto con G9 como con G10. El cierre G10 y sus
desviaciones justificadas se documentan en
[`phase-6-g10-final-report.md`](phase-6-g10-final-report.md). Los invariantes
por loops y el matcher pareado se cierran en G11/G12.

## Veredicto sobre la propuesta actual

El trabajo existente no se desecha: contiene el modelo CSR relacional, la
normalización, encoders, campos, perfiles algebraicos, invariantes globales,
motivos, corpus y una búsqueda exacta de referencia. Sin embargo, no debe
cerrarse como producto en su arquitectura actual.

Los defectos que obligan a reabrir la fase son concretos:

1. la canonización es un método de `FastGraphLabeler`, por lo que la autoridad
   exacta depende arquitectónicamente de una firma finita;
2. la forma canónica incorpora `GraphSignatureId` y la ruta rápida puede ordenar
   por residuos de campo: el mismo grafo puede adquirir identidad distinta al
   cambiar un acelerador;
3. `exact_stable_partition` crea `Vec<Vec<u8>>` y ordena claves por vértice y
   pasada; es una referencia clara, pero no una base de alto rendimiento;
4. la búsqueda individualiza siempre la celda no unitaria más pequeña, explora
   todos sus hijos y serializa cada hoja; no usa trazas, órbitas, automorfismos,
   prefijos ni descomposición estructural avanzada;
5. la cota de memoria solo cuenta arrays retenidos de colores, no arenas,
   claves, ordenaciones ni serializaciones temporales;
6. el nivel adaptativo cuenta únicamente triángulos y `K4` del soporte simple,
   perdiendo dirección, labels, relación, rol, multiplicidad y distribución por
   celdas;
7. las lanes actuales transforman un encoding escalar con constantes afines.
   Si el encoder inicial colisiona, añadir lanes no recupera la información;
8. no existe una fachada única que devuelva `Different`,
   `Isomorphic(mapping)` o `Inconclusive` y verifique el mapping;
9. no existe una forma canónica estable e independiente del perfil, ni parser y
   round-trip contractual de esa forma;
10. las firmas enriquecidas reducen colisiones, pero su nivel de assurance no
    expresa cuándo una igualdad es exacta solo sobre elementos ya codificados.

Por tanto, 1-WL se mantiene como una excelente primitiva de refinamiento. No se
mantiene como centro conceptual del producto y no se sustituye por una única
heurística distinta. El centro será un algoritmo exacto con aceleradores
subordinados.

## Objetivo de producto

Ofrecer dos operaciones distintas y explícitas:

```rust,ignore
pub fn compare(
    &self,
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    budget: ComparisonBudget,
) -> Result<GraphComparison, GraphError>;

pub fn canonicalize(
    &self,
    graph: &IncidenceGraph,
    budget: CanonizationBudget,
) -> Result<CanonicalizationOutcome, GraphError>;
```

```rust,ignore
pub enum GraphComparison {
    Different { witness: DifferenceWitness },
    Isomorphic { mapping: GraphIsomorphism },
    Inconclusive { report: ComparisonReport },
}

pub enum CanonicalizationOutcome {
    Exact { form: CanonicalGraph, report: CanonizationReport },
    Inconclusive { report: CanonizationReport },
}
```

`compare` prioriza el caso habitual: rechaza con invariantes, después intenta un
matching pareado y verifica el mapping. `canonicalize` produce una clave estable
para persistencia, deduplicación o agrupación de más de dos grafos. Comparar dos
grafos no obligará a canonizar ambos si un matcher exacto encuentra y verifica
antes un isomorfismo.

## Contrato matemático

Sea `Encode(G, π)` la serialización del grafo normalizado bajo el orden `π`:

```text
magic | encoding_schema | GraphSchemaId
| número de vértices | número de incidencias | multiplicidad total
| por vértice canónico: kind | frame(label)
| por arco ordenado: source | target | frame(relation) | frame(role) | multiplicity
```

Todos los enteros tienen ancho y endianess fijados; todos los bytes variables
están enmarcados; los arcos se ordenan por la tuple completa. La serialización
es inyectiva sobre `IncidenceGraph` y posee parser estricto sin trailing bytes.

```text
CanonicalBytes(G) = minπ Encode(G, π)
```

Dos grafos del mismo schema son isomorfos si y solo si sus bytes canónicos son
iguales. Un SHA-256 de esos bytes es una clave de índice, no el objeto que se
compara para confirmar igualdad.

## Arquitectura objetivo

```text
IncidenceGraph + GraphSchemaId
              │
              ▼
     GraphPreparation (CSR, diccionarios, workspace)
              │
       ┌──────┴───────────────────────────────┐
       │                                      │
       ▼                                      ▼
FingerprintPipeline                    MicrocanonCore
rechazo / routing / métricas           exacto y perfil-independiente
       │                                      │
       ├─ firmas algebraicas                  ├─ descomposición segura
       ├─ global exacto                       ├─ partición exacta
       ├─ motivos tipados                     ├─ refinadores
       ├─ momentos / patrones                 ├─ IR + poda demostrada
       └─ canales matriciales                 └─ encoding + verifier
              │                                      │
              └──────────── GraphEngine ─────────────┘
                              │
                Different | Isomorphic | Inconclusive
                              o
                    ExactCanonical | Inconclusive
```

Dependencias:

- `canon` puede consumir propuestas de `signature`, pero siempre puede operar
  sin ellas;
- `signature` depende del modelo y de `microfield`, nunca del buscador;
- `encoding` y `verifier` dependen solo del modelo y del schema;
- la fachada coordina estrategias estáticas; no habrá `dyn Trait` en el hot
  path;
- el adapter legacy puede llamar a la fachada, pero ningún módulo nuevo llama
  al canonizador legacy.

## Microcanon v1: algoritmo

### 1. Preparación exacta

1. validar schema, tamaños y presupuesto antes de reservar;
2. construir diccionarios canónicos de kinds, labels y descriptores;
3. preparar CSR de entrada/salida, grados tipados, bucles, multiplicidad,
   soporte y buffers reutilizables;
4. derivar componentes débiles y buckets exactos baratos;
5. no copiar labels ni relaciones por vértice durante el refinamiento.

### 2. Descomposición demostrablemente segura

La primera versión canoniza componentes débiles por separado, ordena sus bytes
exactos y conserva multiplicidades de componentes iguales. Después se añaden,
una a una y detrás de tests diferenciales:

- árboles y bosques relacionales mediante códigos bottom-up exactos;
- bloques y puntos de articulación mediante el árbol bloque–corte con puertos;
- componentes biconexas y componentes idénticas repetidas;
- descomposición modular cuando el módulo y su quotient se certifiquen;
- complemento únicamente para el submodelo simple no dirigido y cuando reduzca
  densidad, sin alterar la forma final del modelo original.

SCC no se canoniza de forma independiente por defecto: las aristas del DAG de
condensación acoplan sus formas. Solo se añadirá una composición que preserve
explícitamente esos puertos.

### 3. Partición inicial exacta

La clave inicial contiene información exacta, no un residuo:

```text
kind, label_id,
grado de entrada/salida por (relation_id, role_id),
multiset de multiplicidades, loops tipados,
componente y descriptores estructurales ya certificados
```

Los fingerprints pueden proponer buckets para ordenar el trabajo. Cada
igualdad o diferencia usada para formar la partición autoritativa se confirma
contra tuples exactas.

### 4. Refinamiento exacto en punto fijo

El refinador base es color refinement relacional dirigido:

```text
key(v) = color(v)
       || multiset_out(color(u), relation, role, multiplicity)
       || multiset_in (color(u), relation, role, multiplicity)
```

La implementación usará IDs internados, arenas planas, counting/radix sort para
enteros de ancho fijo y una cola de celdas activas. No construirá una clave de
bytes heap-owned por vértice y pasada.

El resultado exacto puede:

- ser discreto: se emite el orden y se verifica;
- mantener celdas: se activan refinadores acotados o individualización.

### 5. Refinamiento adaptativo de orden superior

Sobre una celda ambigua de tamaño `a`, el planner estima antes de ejecutar:

- perfiles de distancias acotadas y cortes por celdas;
- catálogo generado de patrones relacionales conectados L0..Ln, graduado por
  número de loops, treewidth, tamaño y coste;
- conteos rooted por vértice y por par de celdas, no solo totales globales;
- 2-WL localizado sobre pares en la celda y su frontera.

No se publica información parcial cuando se agota la cota. 2-WL global solo
será una política explícita para grafos pequeños; no una escalada automática.
Todo color nuevo debe ser una función exacta e isomorphism-invariant del estado.

### 6. Individualización–refinamiento

Si persiste ambigüedad:

1. escoger una celda mediante una estrategia invariante que combine tamaño,
   conectividad con celdas ya fijadas y estimación de splitting;
2. crear un hijo por candidato no eliminado por una órbita demostrada;
3. individualizar, refinar hasta estabilidad y añadir una traza canónica de
   splits y quotient cells;
4. comparar la traza con el mejor camino y podar solo mediante una regla
   demostrada;
5. en una hoja discreta construir `Encode`, comparar con el mínimo y, si dos
   hojas producen la misma forma, extraer y verificar un automorfismo;
6. mantener órbitas bajo el estabilizador del camino para omitir ramas
   equivalentes;
7. terminar únicamente tras cubrir el árbol restante.

El baseline sin poda seguirá compilable en tests. Cada poda se podrá desactivar
para demostrar equivalencia y medir su beneficio aislado.

### 7. Verificación y publicación

Antes de publicar:

- comprobar que `canonical_to_original` y `original_to_canonical` son
  permutaciones inversas;
- reconstruir la serialización desde el mapping y exigir igualdad byte a byte;
- en comparación pareada, aplicar el mapping a todas las tuples exactas;
- calcular `CanonicalGraphKey` solo después de esa verificación;
- adjuntar un informe de ruta, nodos, refinamientos, podas, memoria, tiempo y
  límite consumido.

El informe es auditable, pero no se llamará prueba sucinta de minimalidad. La
minimalidad se sustenta en el árbol completo y la implementación validada.

## Cómo se aprovechan los campos y las firmas

### Rol seguro de los campos

Los campos aceleran evaluación, agregación, batch e incrementalidad. Una
colisión puede dejar de separar dos estados; nunca autoriza una separación
incorrecta ni una forma canónica distinta. El núcleo tendrá una configuración
sin campo que debe producir los mismos bytes.

Las lanes v2 no serán `encode(x) + salt_lane`. Se introducirá una capacidad
interna de domain-separated digest-to-field:

```text
field_element = HashToField(
    GraphAnalysisProfileId || channel || lane || framed(source_bytes)
)
```

Para campos binarios se consumen bytes de expansión del ancho canónico. Para
primos se usa reducción/rejection determinista especificada. SHA se usa aquí
como mixer estable, no se atribuye resistencia a la firma algebraica. Así, las
lanes ven encodings distintos; siguen siendo finitas y colisionables.

Se medirán F251, Goldilocks y GF(2^256):

- F251 maximiza densidad SIMD y memoria mínima;
- Goldilocks ofrece mucho más estado con AVX2 ya disponible;
- GF(2^256) reduce aliasing observado, pero no resuelve equivalencia WL y no se
  supondrá más rápido sin benchmark.

### Assurance explícito para firmas estructurales

Cada familia expondrá su ley y nivel de evidencia:

```rust,ignore
pub enum SignatureAssurance {
    Fingerprint,
    BoundedExactOverEncodedElements { maximum_cardinality: usize },
    ExactTracked,
}
```

`BoundedExactOverEncodedElements` no implica exactitud sobre bytes fuente si el
encoder puede colisionar.

Para un multiconjunto de cardinalidad común `n`:

```text
P_X(t) = product(t + encode(x))
```

La diferencia de dos polinomios mónicos de grado `n` tiene grado a lo sumo
`n-1`. Evaluar en al menos `n` puntos distintos permite demostrar igualdad del
polinomio y, por tanto, del multiconjunto de elementos de campo codificados.
`MultiEvaluationMultisetSignature` expondrá esta cota cuando `K >= n` y los
puntos sean distintos.

Se añadirá `MultiEvaluationSequenceSignature`: Horner en `K` bases distintas,
longitud y potencias de concatenación. Para secuencias de igual longitud `n`,
`K >= n` permite la misma garantía sobre los coeficientes codificados.

### Firmas de grafo reforzadas

`GraphFingerprintPipeline` publicará canales separados, no un digest opaco:

1. **local algebraic v2:** labels y mensajes con lane encoding independiente;
2. **global exact:** conteos, componentes, SCC, grados y registros tipados;
3. **cell moments:** power sums/multievaluación de labels y mensajes por ronda y
   por celda;
4. **loop pattern expansion:** catálogo generado L0..Ln de patrones conectados
   con dirección, clase, relación, rol y multiplicidad; sus conteos son
   aditivos bajo unión disjunta;
5. **matrix channel experimental:** operador relacional ponderado, trazas
   `tr(A^k)` aditivas y evaluaciones de `det(tI-A)` multiplicativas bajo bloques
   diagonales;
6. **canonical digest:** solo tras canonización exacta, fuera de las firmas
   heurísticas.

Los canales matriciales y de motivos pueden separar casos adicionales y extraer
propiedades algebraicas útiles; siguen existiendo grafos cospectrales y patrones
acotados indistinguibles. Ninguno reemplaza el core exacto.

El canal se desarrollará como la
[expansión relacional por loops y Green](relational-green-invariant-research.md).
La semántica de referencia son conteos de homomorfismos de un catálogo generado;
determinante, resolvente y contracciones se investigan como funciones
generadoras o compresiones, inspiradas en el principio computacional de `Theta`
pero demostradas para permutaciones de grafos. Su integración más prometedora
es actualizar node invariants del IR mediante perturbaciones diagonales
rank-one. Es investigación, no un claim de novedad ni una ruta de corrección.

`combine_disjoint` se extiende solo a canales cuya ley se demuestre:

```text
counts(A disjoint-union B)  = counts(A) + counts(B)
connected_patterns(A union B) = patterns(A) + patterns(B)
trace_k(A union B)          = trace_k(A) + trace_k(B)
char_eval(A union B)        = char_eval(A) * char_eval(B)
```

## Estructura física propuesta

```text
src/graph/
├── model.rs
├── schema.rs
├── engine.rs
├── canon/
│   ├── mod.rs
│   ├── budget.rs
│   ├── encoding.rs
│   ├── partition.rs
│   ├── refinement.rs
│   ├── higher_order.rs
│   ├── decomposition.rs
│   ├── search.rs
│   ├── automorphism.rs
│   ├── pair_match.rs
│   ├── verifier.rs
│   ├── workspace.rs
│   └── reference.rs
├── signature/
│   ├── mod.rs
│   ├── local.rs
│   ├── global.rs
│   ├── loop_catalog.rs
│   ├── loop_counts.rs
│   ├── moments.rs
│   └── matrix.rs
├── incremental/
│   ├── mod.rs
│   ├── delta.rs
│   └── workspace.rs
└── legacy.rs
```

Traits internos pequeños:

- `ExactRefiner`: propone una partición equivariante y verificable;
- `TargetCellSelector`: elige trabajo, no decide corrección;
- `NodeInvariant`: produce trazas para orden/poda;
- `Decomposer`: devuelve piezas y una receta exacta de recomposición;
- `SearchPruner`: solo acepta una poda acompañada de su condición demostrada;
- `MappingVerifier`: autoridad final sobre un mapping.

Las estrategias de producción serán genéricas o enums cerrados para permitir
inlining. El canonizador no expondrá factories dinámicas ni plugins de poda.

## Hitos

### F6.G8 — contrato exacto e identidad estable — completado localmente

Entregables:

- `GraphSchemaId`, `CanonicalGraphEncodingId` y `GraphAnalysisProfileId`;
- encoding inyectivo versionado, parser estricto y round-trip;
- `CanonicalGraph`, mappings inversos y verifier;
- `GraphComparison` y resultados fail-closed;
- accesores comprobados para IDs públicos y validación de kinds en hiperaristas;
- deprecación documentada de bytes canónicos ligados a `GraphSignatureId`.

Gate: ninguna operación exacta nueva depende de un tipo de campo o labeler.

Estado del gate: superado. `Microcanon`, encoding, parser, key, mappings y
verifier solo dependen de `IncidenceGraph` y `GraphSchemaId`. El método legacy
delega en esta fachada y ya no incorpora `GraphSignatureId` en los bytes.

### F6.G9 — baseline exacto independiente — baseline completado localmente

Entregables:

- `MicrocanonCore` sin podas avanzadas;
- partición inicial exacta y refinamiento relacional en punto fijo;
- árbol IR completo, mínimo lexicográfico y límites fail-closed de nodos y
  frontier; el presupuesto integral de bytes/tiempo/profundidad pertenece a G10;
- canonizador factorial de referencia para grafos pequeños;
- equivalencia con el canonizador actual donde este termina.

Gate: mismos bytes para todos los perfiles/campos/lanes y todas las
renumeraciones; `Inconclusive` al no completar.

Estado del gate baseline: superado en la suite diferencial hasta seis
vértices, perfiles F251/GF(2^256), encoders, lanes, rondas y renumeraciones. G9
no contiene aún arenas compactas, podas, órbitas ni un presupuesto físico total;
esas propiedades continúan bloqueando el cierre G10.

### F6.G10 — motor de refinamiento y búsqueda industrial — completado localmente

Entregables:

- arenas compactas, IDs internados, active-cell refinement y radix/counting
  sort;
- trazas de refinamiento y node invariants;
- target cell adaptativo;
- extracción y verificación de automorfismos, órbitas y poda por estabilizador;
- prefix/trace pruning con prueba y modo desactivable;
- presupuesto real de memoria, tiempo, nodos y profundidad;
- workspaces reutilizables y cero asignaciones por incidencia tras preparación.

Gate: equivalencia bit a bit con G9 y reducción de al menos 90 % de nodos en el
conjunto adversarial seleccionado, sin regresión superior al 5 % en la ruta
discreta medida.

Estado del gate: superado. La estrategia compacta reproduce G9 en el oráculo
simple exhaustivo hasta orden seis y en 128 grafos relacionales dirigidos con
labels, roles y longitudes hostiles. En `C32` pasa de 97 a 7 nodos, una
reducción del 92,8 %. Criterion mide una mejora de 13,4× a 44,8× en ciclos de
6–12 vértices y de 2,0×–2,2× en la ruta discreta de 64–256 vértices.

Tres medios inicialmente propuestos no se convierten en claims del cierre:

- el refinador usa arenas planas, IDs internados y tuples enteras ordenadas,
  pero no una cola active-cell ni radix sort; el resultado medido ya supera el
  gate y cualquier sustitución futura deberá conservar el orden framed exacto;
- el selector conserva la celda canónica G9 más pequeña. Un selector adaptativo
  experimental cambió bytes y fue rechazado por el contrato de estabilidad;
- la traza se audita y contabiliza, pero no poda. Las únicas podas activas son
  órbitas de automorfismos verificados y el prefijo exacto de vértices.

El presupuesto de bytes cubre buffers retenidos controlados, formas, mappings,
frontera y artefactos de componentes. No pretende medir metadata del allocator,
el grafo de entrada, el resultado entregado al caller ni todos los temporales
atómicos. Por ello la API lo denomina `peak_tracked_bytes`, no memoria RSS.

### F6.G11 — firmas v2 y assurance — completado localmente

Entregables:

- `SignatureAssurance`;
- garantía acotada del multiconjunto multievaluado;
- histograma exacto de grados y correlación multiconjunto multievaluada;
- secuencia multievaluada estática y dinámica;
- domain-separated hash-to-field por lane;
- moments por ronda/celda y patrones conectados tipados;
- `LoopPatternCatalog` generado para L0–L3 y evaluación exacta/comprimida;
- canales matriciales experimentales con leyes de unión disjunta;
- prototipo RG1/RG2, baseline spectrum/zeta/WL y decisión basada en holdout
  sobre la continuidad de la jerarquía relacional de Green;
- comparación F251/Goldilocks/GF(2^256) sin claims probabilísticos inventados.

Gate: cada canal posee ley, identidad, wire, colisión mínima conocida,
metamorfismo, benchmark y clasificación de assurance.

La entrega implementada incluye assurance, multievaluación, lanes
independientes, moments, catálogo inducido conectado L0–L3, compresiones
aditiva/producto, matrix RG1 y theta RG2. El split autenticado de 12.346 clases
n=8 da cero colisiones para el bundle Goldilocks, pero CFI permanece como
contraejemplo. El catálogo general de homomorfismos, zeta y resolventes no se
promocionan aún. Véase el
[informe G11](phase-6-g11-final-report.md) y la
[ADR 0032](adr/0032-g11-assurance-and-field-characteristic.md).

### F6.G12 — descomposición y comparación pareada

Estado: completado localmente.

Entregables:

- árboles/bosques exactos;
- árbol bloque–corte y componentes repetidas;
- matcher pareado inspirado en orden/cortes VF2++, adaptado al modelo
  relacional;
- mapping verificado antes de `Isomorphic`;
- witness concreto del primer canal exacto/finito que demuestra `Different`.

Gate: `compare(G,H)` nunca devuelve igualdad sin mapping verificado y supera a
canonizar dos veces en los corpus de comparación pareada.

La entrega implementada añade prefiltros exactos tipados, block-cut mediante
Tarjan iterativo, interning exacto no recursivo para bosques, refinamiento
conjunto y matcher fail-first. `compare_with_field_profile` calcula el canal
finito dentro de la llamada y solo lo usa como witness negativo. La campaña
determinista coincide con el oráculo canónico en 1024 pares y decide CFI(K4) en
6.976 asignaciones. En caminos, el pareado mejora 8,83× a n=128 y 72,44× a
n=1.024 frente a dos canonizaciones. Véanse el
[informe G12](phase-6-g12-final-report.md) y la
[ADR 0033](adr/0033-paired-comparison-and-long-walks.md).

### F6.G13 — ambigüedad de alta regularidad — completado localmente

Entregables:

- selección adaptativa de orden L0..Ln y tamaño de catálogo por celda;
- patrones rooted tipados por vértice/par y reinyección en refinamiento;
- 2-WL localizado con admisión por coste;
- corpus CFI, strongly regular, cospectral, Miyazaki, grids, hypercubes y
  grafos regulares;
- policy que decide con métricas observables y nunca por el tamaño solamente.

Gate: toda pareja indistinguible por los canales sigue llegando al IR exacto;
ninguna se marca isomorfa por heurística.

Implementado mediante `AdaptiveGraphPipeline`, ceilings fail-closed, catálogo
L0–L3 con skip atómico y `LocalPairRefinementProfile` admitido por `a³·r`.

### F6.G14 — incrementalidad real — completado localmente

Entregables:

- `GraphDelta` transaccional para labels, incidencias, relaciones y
  multiplicidad;
- validación limitada a endpoints y diccionarios afectados;
- recomputación del cono de refinamiento hasta estabilidad;
- invalidación selectiva de firmas globales/motivos;
- estimador que hace fallback temprano a reconstrucción completa;
- igualdad diferencial de estado, firma y análisis frente a recomputar.

Gate: edits locales pequeños reducen trabajo end-to-end, no solo aritmética; a
partir del umbral calibrado el fallback no es peor que insistir localmente.

Implementado con `GraphDelta`, revisión optimista, invalidación tipada,
estimador y tres rutas. En n=1.024, label delta mejora 2,78× frente al rebuild;
la ruta topológica conserva como límite la construcción del CSR candidato.

### F6.G15 — cierre científico e interno — planificado

La ejecución detallada, los niveles de uso admitidos y el artefacto go/no-go se
definen en el [plan G15 interno](phase-6-g15-internal-readiness-plan.md).
G15 cubre primero campos, firmas homomórficas y protocolos derivados; grafos y
canonización son un vertical de consumo, no el producto completo.

Entregables:

- allowlist de campos, encoders y firmas con ley/assurance explícitos;
- API soportada de agregación, secuencias, multisets, tracking y
  multievaluación;
- persistencia de firmas y promoción de reconciliación acotada;
- aplicaciones de firmas comparadas con baselines exactos y criptográficos;
- migración de adapters de química/redes al modelo exacto, sin perder carga,
  aromaticidad, isótopos, quiralidad, dirección o schema de dominio;
- manifiestos de corpus con URL, licencia, SHA-256 y parser versionado;
- oráculos SageMath y nauty/Traces mediante conversión gadget inyectiva del
  modelo relacional a grafo simple coloreado;
- matriz de evidencia, límites, hardware y reproducción;
- guía de uso interno que impide comparar digests como prueba.

Gate: todos los casos de cierre descritos en la sección siguiente.

## Plan de pruebas

### Corrección exhaustiva y diferencial

1. Enumerar todos los grafos simples etiquetados hasta `n=6` y agruparlos por la
   referencia factorial; `Microcanon` debe producir exactamente las mismas
   clases y forma para toda renumeración.
2. Para `n<=9`, muestrear grafos y comparar cada optimización on/off contra la
   referencia; la referencia factorial se ejecuta solo donde el presupuesto lo
   permita.
3. Canonizar los 12.346 representantes no isomorfos de orden 8 del corpus F6:
   debe haber 12.346 bytes canónicos distintos.
4. Ejecutar forma y mapping contra Sage y nauty/Traces en simples coloreados;
   para dirigidos, multigrafos e hipergrafos usar gadgets con prueba de
   preservación y tests de ida/vuelta.
5. Canonizar por completo todos los buckets residuales del perfil adaptativo
   actual, no solo el mínimo y tres pares adversariales.
6. Generar exhaustivamente modelos relacionales pequeños variando dirección,
   loops, multiplicidad, label, role e hiperaristas.

### Metamorfismo

- miles de permutaciones por familia conservan bytes canónicos;
- composición/descomposición y orden de inserción no cambian el resultado;
- cambiar campo, K, rondas, SIMD, paralelismo o planner no cambia bytes;
- `parse(encode(canonical))` reconstruye exactamente el modelo;
- todo mapping publicado preserva cada record exacto;
- presupuesto `b` insuficiente nunca publica forma y aumentar el presupuesto
  no cambia una forma ya completada;
- toda poda activada produce el mismo resultado que la búsqueda sin esa poda.

### Adversarial

- ciclos frente a uniones de ciclos;
- Shrikhande/rook y familias strongly regular;
- CFI de tamaño y base crecientes;
- grafos cospectrales, regulares, vertex-transitive y alta automorfía;
- Miyazaki y familias diseñadas contra IR;
- árboles simétricos, grids, hypercubes, cliques, bicliques y complementos;
- relaciones dirigidas antisimétricas, multiarcos, loops e hiperaristas con
  roles repetidos;
- labels hostiles grandes, diccionarios duplicados, overflows y fallos de
  reserva.

### Fuzzing y robustez

- builder, parser canónico, parser de corpus, `GraphDelta`, verifier y budgets;
- differential fuzz entre referencia, baseline y optimizado;
- ASan, Miri donde aplique, Clippy, doctests compile-fail y panic audit;
- cada bug de canonización se convierte en corpus mínimo permanente.

### Rendimiento

Medir por separado preparación, firma, refinamiento, higher-order, búsqueda,
serialización y verificación. Publicar p50/p95/p99, nodos, hojas, allocations,
peak bytes y tasa de fallback por familia, densidad y simetría.

Gates iniciales:

- la ruta fingerprint fija sigue `O(KR(V+I))` y no regresa más de 5 %;
- la ruta exacta discreta no asigna por incidencia después de preparar;
- G10 reduce al menos 90 % de nodos frente a G9 en el corpus adversarial de
  aceptación;
- el presupuesto de memoria incluye todos los buffers controlados y nunca se
  excede deliberadamente para devolver un candidato;
- `compare` pareado debe superar la doble canonización en mediana y p95;
- se informa el ratio frente a nauty/Traces, pero no se convierte en claim hasta
  medir varias CPUs y familias;
- una optimización que empeore más de 5 % su ruta objetivo o cambie bytes se
  retira, se recalibra o queda explícita.

## Política de datos externos

No se “bajará cualquier dataset” sin control. Cada corpus tendrá finalidad,
schema semántico, licencia compatible, versión, URL, SHA-256, parser y test de
normalización. Los datos grandes viven en caché reproducible y CI programada;
los fixtures mínimos derivados y permitidos viven en Git.

Las categorías de aceptación incluyen:

- atlas y generadores exhaustivos;
- benchmarks adversariales de isomorfismo;
- moléculas con equivalencia química definida;
- redes sociales/biológicas con dirección y labels;
- hipergrafos y knowledge graphs relacionales.

No se mezclan “misma molécula”, “misma topología”, “mismo estereoisómero” y
“mismo grafo de entrada”: cada una necesita un `GraphSchemaId` y un oráculo de
equivalencia propio.

## Migración

1. congelar `canonical.rs` actual como `legacy_canonical_v0` en tests;
2. introducir identidades y encoding sin cambiar firmas existentes;
3. hacer que `MicrocanonCore` produzca forma v1 en paralelo;
4. migrar `CellularGaloisCanonizer` a `GraphEngine` y mantener solo su formato
   adapter donde esté demostrado;
5. deprecar `FastGraphLabeler::canonicalize_exact` tras equivalencia completa;
6. no migrar claves persistidas automáticamente: el cambio de encoding exige
   regeneración o envelope con versión;
7. estabilizar API pública solo después de G15.

## Criterio final de cierre

La fase de grafos no se vuelve a declarar cerrada hasta que:

1. la forma exacta sea independiente de campo, lanes, perfil y heurísticas;
2. todo resultado `Isomorphic` incluya mapping exacto verificado;
3. todos los grafos simples hasta `n=6`, los 12.346 representantes `n=8` y los
   buckets residuales actuales superen el oráculo;
4. dirigidos, labels, multiarcos, loops e hiperaristas tengan campañas propias;
5. cada poda sea diferencialmente equivalente al baseline;
6. CFI, strongly regular y alta simetría terminen o devuelvan `Inconclusive`
   dentro de un presupuesto, nunca un falso positivo;
7. la memoria completa y el tiempo sean presupuestables;
8. firmas y fingerprints tengan leyes, assurance y límites publicados;
9. los adapters de dominio conserven toda la semántica declarada;
10. CI x86-64/AArch64, Miri/ASan, reproducibilidad y benchmarks estén verdes.

Hasta entonces la librería puede usarse internamente para filtrado, firmas
componibles y resultados exactos verificados caso a caso. La igualdad de firmas
o perfiles continúa significando `Indistinguishable`, no isomorfismo.
