# Fase 6.G — etiquetado estructural rápido sobre campos finitos

> Nota posterior (3 de agosto de 2026): este contrato continúa como baseline de
> firmas y etiquetado, pero su cierre de canonización fue reabierto. Microcanon
> v1 ya separa la autoridad exacta de `FastGraphLabeler` en G8–G10; véanse
> [el plan G8–G15](phase-6-canonization-v2-plan.md) y
> [ADR 0031](adr/0031-certified-canonization-core.md). Esta página conserva el
> contrato rápido v0; la forma exacta nueva está en el
> [informe G8/G9](phase-6-g8-g9-implementation-report.md) y el
> [cierre G10](phase-6-g10-final-report.md).

Fecha: 2 de agosto de 2026.

## Decisión

El producto principal no será una búsqueda canónica potencialmente
exponencial. Será un etiquetador y firmador estructural acotado, invariante por
renumeración y lineal por ronda. La canonización exacta queda como capacidad
optativa: el camino rápido solo emite forma canónica cuando sus clases son
unitarias; si persiste simetría devuelve un resultado explícito, no una forma
aproximada presentada como exacta.

Se mantienen separados tres contratos:

| Contrato | Resultado | Complejidad |
|---|---|---|
| `FastGraphLabeler::analyze` | etiquetas y firma con colisiones posibles | `O(K R (V + I))` |
| `try_canonicalize`, partición discreta | bytes Microcanon v1 y permutaciones | análisis rápido más refinamiento exacto sin IR |
| `canonicalize_exact`, simetrías | forma exacta o agotamiento explícito | búsqueda opt-in acotada, exponencial en peor caso |

`I` es el número de incidencias dirigidas normalizadas, `R` las rondas y `K`
las evaluaciones independientes.

## Modelo exacto de entrada

`IncidenceGraph` es un multigrafo dirigido relacional almacenado como CSR de
salida y entrada. Conserva exactamente:

- tipo y etiqueta de cada vértice;
- dirección;
- etiqueta de relación y rol/puerto;
- bucles;
- multiplicidad `u64`;
- identidad de hiperaristas.

Duplicados idénticos se comprimen sumando su multiplicidad con overflow
comprobado. Los pools de etiquetas y descriptores se ordenan e internan de
forma determinista. Las rondas consultan slices prestados y no construyen un
`Vec` por vecindario.

Una hiperarista de aridad `a` se representa por un vértice auxiliar y `2a`
incidencias dirigidas. No se expande a una clique de `a(a-1)` arcos, por lo que
se preserva la semántica y el coste permanece lineal en la aridad.

`from_legacy_topology` migra el contrato variable/cláusula histórico sin
aplastar cláusulas. Los estados iniciales legado siguen siendo etiquetas de
compatibilidad con posibles colisiones; las aplicaciones nuevas deben entregar
los bytes fuente exactos mediante `IncidenceGraphBuilder`.

## Recurrencia algebraica

Cada lane posee constantes derivadas y ligadas a `GraphSignatureId`:

- sal de lane;
- multiplicador de etiqueta vecina;
- offset de multiconjunto;
- separadores de entrada y salida;
- base de actualización;
- base del transcript;
- offset de agregación global.

Para una incidencia se forma un mensaje afín a partir de etiqueta vecina,
relación, rol y dirección. El vecindario se acumula como producto en un punto
afín. Las multiplicidades comprimidas usan potencia, con ruta directa para
multiplicidad uno. Los factores cero se cuentan y se mezclan de forma
separada.

La etiqueta siguiente combina mediante Horner:

```text
etiqueta anterior
salida: producto + ceros
entrada: producto + ceros
dominio de ronda
```

Después de la ronda se agrega conmutativamente el multiconjunto completo de
etiquetas. Las agregaciones de las rondas `0..R` se encadenan en un transcript
ordenado. La firma final incluye además números exactos de vértices,
incidencias, multiplicidad y rondas ejecutadas.

El perfil fijo conserva internamente producto no nulo y contador de ceros para
cada lane y cada ronda. `combine_disjoint` suma contadores, multiplica productos
y reconstruye el transcript. Por tanto:

```text
signature(A ⊔ B) == combine_disjoint(signature(A), signature(B))
```

La ley se ofrece únicamente para `Fast`: dos ejecuciones `Robust` pueden parar
en rondas distintas y no representan automáticamente el mismo calendario. Las
identidades y todos los overflows se comprueban antes de publicar el resultado.

## Huella híbrida SHA-256

`analyze_hybrid` conserva dos canales distintos:

```text
FastGraphSignature<F, K>       ley algebraica y composición
InvariantGraphDigest(SHA-256)  discriminación global no homomórfica
```

El SHA-256 no se calcula sobre los índices ni solamente sobre la firma de
campo. Consume información adicional:

- histograma ordenado de etiquetas de cada ronda;
- tipo, etiqueta exacta y etiqueta estructural final de cada vértice;
- multiconjunto ordenado de relaciones entre etiquetas estructurales;
- dirección, relación, rol y multiplicidad exactos.

Cada entrada se enmarca y se resume antes de ordenar; la raíz incluye
`GraphSignatureId` y los contadores exactos. Por ello una renumeración conserva
el digest. Si dos grafos ya generan exactamente el mismo descriptor refinado,
SHA-256 no los separará; tampoco convierte refinamiento local en canonización.

El modo híbrido es opcional porque ordenar y resumir todas las entradas añade
`O((V + I) log(V + I))` y trabajo SHA. El perfil puramente algebraico conserva
su coste lineal por ronda. El digest SHA no se combina homomórficamente; cada
canal mantiene deliberadamente su contrato independiente.

## F251 y campos generados

F251 no se conserva como un módulo espectral aislado: se convierte en una
especialización de primer nivel del algoritmo completo:

```rust
use homomorphic_hash_rs::{F251GraphLabeler, RefinementProfile};

let labeler = F251GraphLabeler::<3>::f251(RefinementProfile::fast())?;
# Ok::<(), homomorphic_hash_rs::GraphError>(())
```

Sus ventajas son layout de un byte, aritmética barata y backends batch AVX2 ya
certificados en `microfield`. Su desventaja es el espacio reducido de cada lane;
por ello el perfil recomendado usa varias lanes independientes.

El mismo tipo acepta cualquier campo estático mantenido o generado que
implemente `Field + CanonicalEncoding + StaticField + Pow`, suministrando el
encoder de su familia:

- `PrimeIntegerEncoder` para primos, incluido `Fp251V1`;
- `BinaryPolynomialEncoder` para extensiones binarias;
- un `StructuralEncoder<F>` nominal para una familia futura.

`GraphFieldParameters::new` permite estudiar propuestas con puntos y bases
explícitos. Todos esos valores forman parte de `GraphSignatureId`: resultados
de experimentos incompatibles no se comparan accidentalmente.

La suite integra un campo GF(2^9) generado durante el build para demostrar que
el motor no depende de una lista hardcodeada de campos.

## Perfiles

### `Fast`

- número exacto y pequeño de rondas;
- sin ordenación de particiones dentro del bucle;
- trabajo lineal y predecible;
- perfil recomendado para grafos grandes.

### `Robust`

- mínimo y máximo explícitos;
- calcula la partición inducida entre rondas;
- termina cuando la relación de equivalencia deja de cambiar;
- nunca entra en búsqueda no acotada.

El número de lanes `K` es const-genérico e independiente del perfil. Aumentarlo
mejora discriminación a costa proporcional de memoria y multiplicaciones.

## Garantías y límites

Se garantiza, bajo un mismo `GraphSignatureId`:

```text
analyze(G).signature == analyze(relabel(G, permutation)).signature
label_G[v] == label_relabel_G[permutation(v)]
```

También se preservan dirección, roles, multiplicidad e hiperaristas en los
mensajes. No se garantiza la implicación inversa: dos grafos no isomorfos
pueden colisionar en un campo finito.

`TryCanonicalOutcome` tiene dos resultados cerrados:

- `Canonical`: la partición es discreta y se emiten bytes exactos;
- `SymmetryRemaining`: se devuelve el análisis rápido completo y no se afirma
  canonización.

Los vértices estructuralmente simétricos deben conservar la misma etiqueta. No
se introduce el índice original, aleatoriedad de proceso ni Bloom dependiente
de la numeración para forzar unicidad falsa.

### Diagnóstico de degeneración

`diagnose_degeneracy` contrasta la partición finita con una 1-WL de bytes
exactos. Si la exacta divide una clase finita, informa aliasing del campo. Si la
exacta continúa siendo no discreta, informa ambigüedad local; más campos o
rondas no se ofrecen como garantía. `is_highly_regular` activa una señal de
routing v1 cuando `V ≥ 4`, al menos 75 % de los vértices permanecen ambiguos y
la mayor clase contiene al menos 25 % del grafo.

`MultiFieldGraphEvidenceBuilder` agrupa perfiles heterogéneos y deriva una
identidad de todos sus `GraphSignatureId`. El resultado de comparación se llama
`Different` o `Indistinguishable`, nunca isomorfo.

`canonicalize_exact` escala automáticamente desde una partición rápida discreta
a individualización–refinamiento exacto solo dentro del presupuesto entregado.
Sus resultados son `Exact` y `BudgetExhausted`; el segundo no contiene forma
parcial.

## Evidencia actual

`tests/fast_graph.rs` cubre:

- normalización transaccional y compresión de duplicados;
- F251 con tres lanes;
- GF(2^256) y Goldilocks;
- campo externo generado GF(2^9);
- composición exacta de dos componentes desconectadas;
- 64 renumeraciones pseudoaleatorias deterministas;
- dirección, rol y multiplicidad;
- bucles e hiperaristas;
- adaptación del proveedor legado;
- forma canónica discreta y rechazo explícito de un ciclo simétrico;
- colisión forzada por palomar en F251/K=1 que el descriptor SHA separa;
- separación de identidad por campo, perfil y parámetros.
- equivalencia owned/prepared, robust/híbrida y rechazo de planes incompatibles;
- cero asignaciones después de reservar el workspace secuencial;
- igualdad byte a byte de AoS secuencial, rangos Rayon con 2–4 hilos y
  SoA/batch F251 sobre 24 multigrafos pseudoaleatorios adversariales;
- ejecución AVX2 real cuando la CPU la ofrece y fallback portable en el resto;
- equivalencia directa entre `CellularGaloisCanonizer::try_analyze` y
  `F251GraphLabeler` sobre el grafo adaptado.

`tests/graph_canonical.rs` añade aliasing F251, ciclos desconectados
indistinguibles, Shrikhande/torres 4×4, 128 normalizaciones, 64 renumeraciones,
ambos límites de presupuesto y los 1.088 grafos simples de cuatro y cinco
vértices comparados con enumeración exhaustiva independiente. SageMath 10.7
confirma además 35 pares de ciclos y el par fuertemente regular.

Miri ejecuta además la normalización transaccional, la composición disjunta y
la ruta híbrida completa, incluida la búsqueda determinista de la colisión
F251, sin diagnósticos. La búsqueda exacta y sus dos salidas presupuestarias
también pasan un test focalizado bajo Miri.

`benches/fast_graph.rs` compara preparación, propiedad del resultado, AoS/SoA,
AVX2, paralelismo, SHA-256 y GF(2²⁵⁶), con tres lanes y cuatro rondas. El
throughput cuenta visitas reales a registros CSR: salida + entrada en cada
ronda. No equivale al número de multiplicaciones de campo.

Medición Criterion `--quick` del 2 de agosto de 2026: Intel i7-13700HX,
24 hilos lógicos, Linux x86-64, AVX2, `rustc 1.96.0-nightly`, release LTO. Los
intervalos son exploratorios y no constituyen promesas para otro host:

| Perfil | 1.024 vértices | 16.384 vértices | 131.072 vértices |
|---|---:|---:|---:|
| F251 owned, incluye `prepare` | 80,2–80,6 M visitas/s | 74,6–75,2 M | 79,4–80,4 M |
| F251 prepared, AoS, 1 hilo | 114,3–114,6 M | 110,1–110,6 M | 110,7–111,3 M |
| F251 prepared, AoS + Rayon | 135,3–139,5 M | 373,1–377,7 M | 466,7–490,0 M |
| F251 prepared, SoA + AVX2, 1 hilo | 124,4–125,8 M | 127,2–131,2 M | 123,9–125,5 M |
| F251 SoA + AVX2 + Rayon CSR | 129,2–131,2 M | 221,2–227,5 M | 369,2–377,8 M |
| F251 híbrido SHA-256 | 21,1–21,4 M | 21,7–21,9 M | 21,0–21,2 M |
| GF(2²⁵⁶) prepared, 1 hilo | 338–344 K | 340–343 K | no ejecutado |

Conclusiones de promoción:

- `PreparedGraph` elimina encoding repetido y precalcula la parte afín de cada
  descriptor/dirección/lane;
- `GraphWorkspace` conserva labels, siguiente ronda, particiones, mapas y
  agregados; el camino prestado secuencial no asigna tras reservar;
- SoA+AVX2 sí mejora el escalar de un hilo entre aproximadamente 9 % y 16 % en
  este corpus, por lo que se ofrece mediante `F251BatchGraphWorkspace`;
- AoS+Rayon es mucho más rápido en grafos grandes de este host. Combinar Rayon
  con el bridge SoA pierde frente a AoS paralelo, así que no se selecciona SIMD
  automáticamente;
- `GraphExecution::parallel()` usa un umbral conservador de 1.024 vértices y
  permite sustituirlo explícitamente; la política no altera `GraphSignatureId`;
- SHA-256 sigue siendo opt-in y GF(2²⁵⁶) se reserva para perfiles donde su mayor
  codominio compense el coste.

## Cierre F6.G3–G6

F6.G3 queda cerrado. La migración legado también forma parte del gate:
`CellularGaloisCanonizer` ya no posee recurrencia, espectro ni álgebra propios;
adapta la topología y llama al motor F251. Las aserciones que codificaban
exactamente la recurrencia antigua fueron reemplazadas por contratos de
identidad, límites, hiperaristas y equivalencia directa con el motor nuevo.

F6.G4 queda también cerrado. `IncrementalGraphState` conserva las capas
`0..R`, agregados, orden y un CSR simétrico de dependencias. La entrada nueva se
audita completa; la aritmética se limita al cono de propagación y la
publicación es transaccional. Inserciones y retiradas de aristas unen o separan
componentes exactamente. La partición persistente se actualiza en
`O(V + C log C)`, donde `C` son las etiquetas finales cambiadas. Véase
[phase-6-g4-final-report.md](phase-6-g4-final-report.md).

F6.G5 separa aliasing aritmético de degeneración local, incorpora evidencia
multi-campo identificada y contrasta el corpus con un oráculo exhaustivo y
SageMath. F6.G6 añade búsqueda exacta no recursiva con presupuesto de nodos y
estado retenido. La colisión `C6`/`2C3` demuestra que ni tamaño, rondas, campos
adicionales ni SHA resuelven por sí solos una regularidad local exacta. Véase
[phase-6-g5-g6-final-report.md](phase-6-g5-g6-final-report.md). Con ello la
Fase 6 queda cerrada.
