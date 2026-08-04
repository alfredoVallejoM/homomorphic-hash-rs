# Informe F6.G11 — firmas v2, patrones y expansión Green acotada

Fecha: 3 de agosto de 2026.

Estado: vertical de ingeniería completado y validado; la investigación de
resolventes, zeta y catálogos generales de homomorfismos continúa abierta.

## Resultado ejecutivo

G11 refuerza las firmas sin alterar la autoridad exacta de `Microcanon`. Se han
incorporado:

- assurance explícito para toda firma estructural;
- secuencias multievaluadas estáticas y dinámicas;
- hash-to-field independiente y separado por lane;
- histogramas exactos de grado con correlación multiconjunto multievaluada;
- moments por celda;
- catálogo exacto de patrones relacionales inducidos conectados L0–L3;
- compresión aditiva y multiplicativa de sus conteos;
- canal matricial RG1 con trazas y evaluaciones características;
- prototipo RG2 con seis contracciones theta;
- campaña discovery/holdout sobre las 12.346 clases no isomorfas de orden ocho.

El bundle Goldilocks `pattern-product + matrix + theta` no presenta colisiones
en ninguna de esas 12.346 clases. Este resultado no demuestra completitud: el
par CFI(K4) congelado sigue colisionando en todos los canales G11 y solo la ruta
exacta puede decidirlo.

La evidencia reproducible está en
[`g11-v1.json`](../../validation/f6/results/g11-v1.json).

## Contratos incorporados

### Assurance

`SignatureAssurance` distingue tres afirmaciones:

| Clase | Qué permite afirmar |
|---|---|
| `Fingerprint` | igualdad de un estado finito y colisionable |
| `BoundedExactOverEncodedElements { maximum_cardinality }` | igualdad de coeficientes ya codificados bajo la cota de interpolación |
| `ExactTracked` | los valores fuente se conservan y comparan exactamente |

`BoundedExactOverEncodedElements` nunca eleva un encoder colisionable a una
prueba sobre bytes fuente. `MultisetSignature`, `SequenceSignature`, channels
de grafo, moments, matrices y theta se clasifican como `Fingerprint`.

### Secuencia multievaluada

`MultiEvaluationSequenceSignature<F,E,K>` evalúa por Horner en `K` bases
distintas, no cero y no uno. Conserva longitud, concatenación y wire `MFSG`.
Para dos secuencias de igual longitud `n <= K`, la igualdad de las `K`
evaluaciones determina el polinomio de coeficientes de campo. La variante
`DynamicMultiEvaluationSequenceSignature` produce el mismo wire bajo el mismo
`FieldId`, encoder y bases.

### Lanes independientes

`DomainSeparatedHashToFieldEncoder<K>` aplica expansión SHA-256 y rejection
canónico por campo. Cada coordenada liga:

```text
profile_id | channel | lane | length | source | attempt | block
```

Los límites defensivos de input e intentos no cambian `EncoderId`; perfil,
canal, número de lanes y algoritmo sí. SHA-256 actúa como mixer estable, no
convierte la firma algebraica en una firma criptográfica.

## Canales de grafo

### Histograma de grado y firma de multiconjunto

`DegreeHistogramProfile<F,E,K>` materializa de forma dispersa el histograma
exacto `grado -> número de vértices`. En un grafo simple, no dirigido y sin
bucles, `support`, `outgoing_records` e `incoming_records` coinciden con el
grado ordinario. En el modelo relacional se conservan por separado:

- vecinos débiles distintos, sin contar el propio vértice;
- registros CSR salientes y entrantes, con cada bucle una vez;
- suma exacta de multiplicidades salientes y entrantes.

Los histogramas son invariantes exactos y pueden rechazar sin riesgo de
colisión. Una `MultiEvaluationMultisetSignature` adicional correlaciona por
vértice esos cinco valores, kind y estadísticas de loops. Esta parte compacta
sigue siendo `Fingerprint`. El perfil suma exactamente bajo unión disjunta,
posee wire `MFDH` y liga campo, encoder y puntos de evaluación a su identidad.

El comparador G12 incorpora además el descriptor exacto conjunto antes de
Tarjan: caminos y estrellas con igual número de vértices/aristas se rechazan
con cero nodos de búsqueda.

Criterion `--quick` sobre ciclos, release LTO e Intel Core i7-13700HX midió
15,82 µs para 64 vértices, 246,87 µs para 1.024 y 3,94 ms para 16.384: alrededor
de 4,15 millones de vértices por segundo y escalado lineal en esta familia.

### Moments por celda

`CellMomentProfile<F,K,D>` acumula, por descriptor exacto de celda, cardinalidad
y power sums `sum encode(value)^d`. La API genérica admite rondas posteriores
si el caller proporciona descriptores equivariantes. `analyze_initial` ofrece
una ruta segura basada en kind, label y registros relacionales de un salto.
Los moments suman por unión disjunta.

El holdout muestra que este canal aislado es débil en grafos simples regulares:
941 salidas para 6.177 clases. Se conserva como feature local barata, no como
digest global principal.

### Catálogo inducido conectado L0–L3

`LoopPatternCatalog::l0_to_l3()` enumera todos los subconjuntos conectados de
hasta cuatro vértices y canoniza exactamente cada patrón pequeño probando como
máximo 24 permutaciones. El descriptor incluye:

- kind y label de cada vértice;
- dirección de cada arco;
- relación, rol y multiplicidad;
- orden y rango ciclomático L0–L3.

El preflight `sum choose(n,k) * k!` es invariante. Si rebasa el presupuesto, el
resultado es `SkippedBudget` y no contiene conteos parciales. Los conteos de
patrones conectados suman exactamente bajo unión disjunta.

Este catálogo es de subgrafos inducidos, no el catálogo general de conteos de
homomorfismos propuesto por el track RGI. Esa extensión permanece abierta.

### Dos compresiones de patrones

`PatternFieldFingerprint` suma `count * encode(pattern)`. Es barata y adecuada
en primos grandes, pero en característica dos reduce las multiplicidades a
paridad.

`PatternProductFingerprint` evalúa el multiconjunto de patterns de forma
multiplicativa. Calcula potencias por multiplicidad y mantiene por lane tanto el
producto no cero como el número exacto de factores cero. Su composición para
unión disjunta es total y multiplicativa. En GF(2²⁵⁶) recuperó exactamente la
partición observada del catálogo que la suma aditiva había perdido.

La compresión sigue siendo un fingerprint: no retiene el catálogo fuente ni
prueba isomorfismo.

### RG1 matricial

`RelationalMatrixProfile<F,K>` construye por lane un operador equivarante:

```text
A[source,target] += multiplicity * encode(relation, role)
A[vertex,vertex] += encode(kind, label)
```

Una renumeración produce `P A P^-1`. El canal publica:

- `trace(A^k)` para `k=1..=maximum_trace_power`;
- `det(t_lane I - A_lane)` mediante eliminación gaussiana de referencia.

Sobre unión disjunta, las trazas suman y los determinantes multiplican. El
preflight denso se decide antes de reservar o calcular, y un skip no expone
valores parciales.

### RG2 theta

`RelationalThetaProfile<F,K>` congela seis triples:

```text
(1,2,2), (1,2,3), (1,3,3), (2,2,2), (2,2,3), (2,3,3)
```

Para cada uno calcula:

```text
Theta_(a,b,c) = sum_(u,v) A^a[u,v] * A^b[u,v] * A^c[u,v]
```

Son contracciones cerradas equivariantes que representan patrones theta con
dos loops independientes. Suman bajo bloques diagonales y poseen presupuesto
atómico, identidad y wire `MFTH`. No implementan todavía resolventes ni
actualizaciones Sherman–Morrison.

## Campaña discovery/holdout

El corpus certificado contiene una representación por clase de isomorfismo de
los 12.346 grafos simples de orden ocho. La partición se fijó antes de medir:

```text
SHA-256(graph6)[0] bit 0: 0 = discovery, 1 = holdout
```

- discovery: 6.169 grafos;
- holdout: 6.177 grafos;
- 49 relabelings inversos distribuidos entre ambos splits;
- cero fallos de invariancia.

Resultados principales:

| Canal | Discovery: distintas/6.169 | Holdout: distintas/6.177 | Máx. bucket holdout |
|---|---:|---:|---:|
| F251 fast / 1-WL | 6.088 | 6.114 | 5 |
| moments Goldilocks | 952 | 941 | 109 |
| patterns exactos L0–L3 | 5.895 | 5.878 | 3 |
| patterns aditivos F251 | 5.894 | 5.878 | 3 |
| patterns aditivos Goldilocks | 5.895 | 5.878 | 3 |
| patterns aditivos GF(2²⁵⁶) | 509 | 505 | 77 |
| patterns producto GF(2²⁵⁶) | 5.895 | 5.878 | 3 |
| matrix F251/Goldilocks | 5.955 | 5.937 | 3 |
| matrix GF(2²⁵⁶) | 16 | 16 | 763 |
| theta RG2 F251/Goldilocks | 6.163 | 6.170 | 2 |
| theta RG2 GF(2²⁵⁶) | 1 | 1 | 6.177 |
| closed walks Goldilocks 8/16/64/10¹² | 5.955 | 5.937 | 3 |
| bundle Goldilocks patterns+matrix+theta | 6.169 | 6.177 | 1 |

La igualdad de F251 y Goldilocks en varios perfiles indica que, en este corpus,
la colisión dominante es estructural y no de campo. La única colisión extra de
F251 en discovery para patterns confirma que un primo pequeño sí puede añadir
aliasing finito.

Los canales aditivos, trazas simétricas y theta pueden cancelarse masivamente en
característica dos. El ancho de GF(2²⁵⁶) no cambia su característica. Por ello:

- los campos binarios siguen siendo apropiados para secuencias, productos de
  multiconjunto y compresión multiplicativa;
- F251/Goldilocks son los perfiles preferidos para conteos aditivos, traces y
  contracciones de grafos no dirigidos;
- el perfil de campo forma parte de la identidad y nunca se selecciona como si
  todos los canales tuviesen la misma semántica discriminante.

## Familias adversariales

| Familia | 1-WL | Matrix | Patterns | Theta | Bundle G11 |
|---|---:|---:|---:|---:|---:|
| `C6` frente a `2C3` | no | sí | sí | sí | sí |
| Shrikhande frente a rook 4×4 | no | no | sí | no | sí |
| CFI(K4) par frente a twist | no | no | no | no | no |

El último caso impide declarar completa cualquier combinación G11.
`Microcanon` continúa siendo la única autoridad exacta.

## Rendimiento observado

Criterion `--quick`, release con LTO, Intel Core i7-13700HX, x86-64 y Rust
1.96-nightly. Son mediciones locales orientativas, no umbrales portables:

| Canal | n=8 | n=16 | n=32 |
|---|---:|---:|---:|
| moments Goldilocks K3 D4 | 10,13 µs | 21,72 µs | 43,16 µs |
| catálogo inducido L0–L3 | 216,19 µs | 603,70 µs | 4,28 ms |
| matrix trace4 + char K3 | 42,91 µs | 163,91 µs | 995,44 µs |
| theta RG2 Goldilocks K3 | 22,85 µs | 106,96 µs | 652,17 µs |
| compresión aditiva del catálogo ya calculado | 3,07 µs | 3,10 µs | 3,09 µs |
| compresión producto del catálogo ya calculado | 7,77 µs | 7,75 µs | 7,75 µs |

El catálogo inducido escala combinatoriamente y debe conservar su presupuesto.
Theta es muy discriminante en n=8 y más barato que el catálogo en estos tamaños,
pero usa matrices densas y no es una ruta para grafos masivos. Moments es el
único de estos tres canales con comportamiento prácticamente lineal.

## Gates superados

- invariancia por relabeling sobre campos F251, Goldilocks y GF(2²⁵⁶);
- composición por unión disjunta verificada contra cálculo directo;
- skip atómico por presupuesto;
- identidad estable ligada a campo, encoder, lane count, catálogo y profundidad;
- wires diferenciados `MFPC`, `MFPF`, `MFPP`, `MFCM`, `MFRM` y `MFTH`;
- estático/dinámico wire-compatible para la secuencia multievaluada;
- prueba exhaustiva pequeña de la cota multievaluada;
- Clippy sin warnings, tests del workspace y benchmarks compilables;
- corpus discovery/holdout reproducible con adversariales separados.

## Trabajo deliberadamente pendiente

G11 no introduce:

- un catálogo general generado de homomorfismos relacionales;
- Ihara/Bartholdi zeta;
- determinantes o resolventes simbólicos multivariantes;
- inversion densa, Sherman–Morrison o Woodbury dentro del IR;
- 2-WL localizado;
- una prueba de anterioridad o novedad científica.

Esas líneas solo continuarán si superan los baselines congelados de este informe
en un corpus futuro no usado para seleccionar parámetros.

## Extensión de cierre G11

El cierre G12 incorpora dos extensiones sin modificar los baselines anteriores:

- `StaticGraphFieldProfile` y `DynamicGraphFieldProfile` publican una policy
  común por característica. Un campo runtime GF(p) o GF(2^m) puede exportarse y
  regenerarse como tipo monomorfizado conservando `FieldId`;
- `RelationalClosedWalkProfile` separa longitud de closed walk y rango
  ciclomático. Consulta exponentes `u64` mediante Berlekamp–Massey y una
  recurrencia garantizada por Cayley–Hamilton, con wire `MFCW`, composición por
  unión disjunta y presupuesto atómico.
- `ClosedWalkOperator::NonBacktracking` aplica la misma recurrencia sobre
  estados de incidencia y prohíbe volver inmediatamente al origen. En árboles
  produce trazas cero; en ciclos conserva recorridos cerrados sin rebote.

Las consultas 1..16 coinciden con el canal matricial denso y los tests saltan a
longitud 10^12 sobre F251, Goldilocks, GF(2^256) y un campo externo GF(2^9).
Esto no convierte el perfil en un contador de ciclos simples.

El holdout ampliado autentica 19 canales. El perfil de grado produce 941
salidas en 6.177 clases, 77.946 pares colisionantes y bucket máximo 109. Es un
prefiltro lineal útil, pero no separa pares regulares como C6/2C3, SRG o CFI,
tal como exige el modelo de assurance. Closed walks Goldilocks produce 5.937
salidas y bucket máximo 3: aporta información útil, pero no mejora por sí solo
el bundle existente ni separa CFI. SHA-256 del informe regenerado:
`db7a166f0b8882f82a0677ff54b7d07c6825fa69673700f7c914d905f9ea1602`.

G12 queda cerrado en
[`phase-6-g12-final-report.md`](phase-6-g12-final-report.md).
