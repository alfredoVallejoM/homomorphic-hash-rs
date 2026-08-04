# Investigación F6.RG — expansión relacional por loops y funciones de Green

Fecha: 3 de agosto de 2026.

Estado: propuesta de investigación; no implementada y sin claim de novedad.

## Motivación

El invariante de nudos `Theta=(Delta, theta)` de Bar-Natan y van der Veen sigue
un patrón atractivo para este proyecto:

1. construye una matriz local/equivariante a partir del diagrama;
2. su determinante recupera un invariante clásico;
3. la inversa actúa como función de Green o tráfico entre posiciones;
4. contracciones de varias copias de esa función producen un polinomio mucho
   más discriminante;
5. la formulación mantiene coste polinómico y se puede evaluar sin expandir
   siempre el polinomio completo.

La traducción literal de sus fórmulas no tiene sentido: dependen de cruces,
orientación y movimientos de Reidemeister propios de nudos. Sí tiene sentido
trasladar el principio de diseño a grafos relacionales.

Nombre provisional del track: `RelationalGreenInvariant` (RGI). No se usará
`GraphTheta` en API ni publicaciones hasta aclarar relación matemática,
nomenclatura y novedad.

## Traducción combinatoria precisa

Para grafos existe una base más directa que la analogía visual. Si `A_G` es la
adyacencia de `G` y `F` un patrón, la contracción completa

```text
hom(F,G) = sum_phi product_(u,v in E(F)) A_G[phi(u), phi(v)]
```

cuenta homomorfismos de `F` hacia `G` y es invariante bajo renumeración. En el
modelo relacional, cada factor añade dirección, relación, rol, kind y
multiplicidad. Esta será la semántica de referencia; determinantes y
resolventes se estudiarán como formas de generar o comprimir familias de estas
contracciones.

Los patrones se graduarán por número ciclomático
`beta(F)=|E(F)|-|V(F)|+componentes(F)` y por treewidth:

| Grado | Base de patrones | Límite conocido |
|---|---|---|
| L0 | árboles/bosques | todos los conteos de árboles tienen el mismo poder de separación que color refinement/1-WL |
| L1 | patrones unicíclicos | añade ciclos decorados, pero sigue siendo incompleto |
| L2 | theta, barbell y figure-eight | correlaciona dos ciclos/flujo entre regiones |
| L3+ | `K4` y patrones superiores | añade interacciones que pueden sobrevivir a L2 |

Triángulos y `K4` dejan así de ser excepciones escritas a mano. Un generador
canónico producirá un catálogo versionado `LoopPatternCatalog<L,D>` hasta orden
`L` y tamaño/coste `D`. `K4` pertenece a L3, no a L2. Ningún `L` fijo es
completo; familias CFI y otros contraejemplos obligan a conservar el exacto.

La literatura ya relaciona conteos de patrones de treewidth `k` con `k`-WL y
demuestra que todos los conteos de homomorfismos caracterizan el isomorfismo de
grafos finitos. La aportación potencial no es redescubrir esos teoremas, sino
una base relacional generada, evaluada sobre `microfield`, componible e
integrada adaptativamente en un IR certificado.

## Operador relacional

Para cada descriptor dirigido `r=(relation, role, direction)` se construye una
matriz de adyacencia `A_r`. Kinds y labels de vértice producen matrices
diagonales `D_l`. Con indeterminadas o evaluaciones identificadas:

```text
M_G(z, x, y) = z I
               - sum_r x_r A_r
               - sum_l y_l D_l
               - Q_G
```

`Q_G` puede contener grados, loops y multiplicidades mediante una convención
versionada. Para un cambio de numeración con matriz de permutación `P`:

```text
M_relabel(G) = P M_G P^-1
```

Por tanto:

```text
Delta_G = det(M_G)
R_G = M_G^-1

det(P M P^-1) = det(M)
R_relabel(G) = P R_G P^-1
```

El determinante, las trazas y cualquier contracción completa de índices con
tensores del propio grafo son invariantes por renumeración.

## Jerarquía propuesta

### RG0 — datos exactos y quotient

Conteos, componentes, grados tipados, loops, multiplicidades y quotient de la
partición. Es el nivel actual exacto y barato.

### RG1 — un loop

- `det(M_G)` o evaluaciones del polinomio característico/generalizado;
- `trace(A^k)` y moments de Krylov;
- Ihara/Bartholdi zeta para el submodelo donde la semántica sea exacta;
- diagonales y sumas de filas/columnas de `R_G(z)` como labels equivariantes.

Este nivel ve ciclos y espectro, pero colisiona en grafos cospectrales.

### RG2 — patrones y contracciones de dos loops

Primero se calculan conteos exactos de una base pequeña de patrones theta,
barbell y figure-eight. Después se evalúan varias resolventes `R_G(z_1)`,
`R_G(z_2)` y se contraen con matrices
de relación, diagonales de color e incidence tensors. Esquema general:

```text
I_D(G) = Contract_D(
    A_r, D_l, R(z_1), R(z_2), ..., delta
)
```

`D` es un diagrama cerrado versionado: cada índice de vértice se suma y ninguna
posición original queda libre. Un ejemplo de familia, no una fórmula congelada:

```text
1^T ((R1 hadamard R2) A_r R3) 1
```

Los productos matriciales cerrados ordinarios tienden a colapsar a trazas y
datos espectrales. Por eso el espacio candidato incluye productos Hadamard,
diagonales, incidence tensors tipados e identificaciones de índices con
valencia mayor que dos. Se buscarán contracciones que no colapsen
algebraicamente al espectro o generalized spectrum. La prueba simbólica de esa
no-redundancia es parte del track.

Una familia especialmente clara para grafos no dirigidos es

```text
Theta_(a,b,c)(G) = sum_(u,v)
    (A^a)[u,v] * (A^b)[u,v] * (A^c)[u,v].
```

Cuenta homomorfismos de un patrón theta con tres caminos. La versión de
resolvente agrega muchas longitudes en una función racional. Ambas pueden
colisionar; incluso grafos fuertemente regulares con los mismos parámetros
pueden compartir contracciones que colapsen a su álgebra de adyacencia.

### RGk — patrones conectados

Contracciones de orden fijo forman una jerarquía de invariantes. Se estudiará su
relación con:

- conteos de homomorfismos y patrones de treewidth acotado;
- k-WL y refinamiento relacional;
- funciones zeta y espectros generalizados;
- quantum walks de una y varias partículas;
- invariant theory de la acción del grupo simétrico.

No se afirma que un `k` fijo sea completo. La colección de todos los patrones
puede ser completa en universos finitos, pero su número/coste deja de ser el
invariante compacto buscado.

## Encaje con `microfield`

Hay tres modos de cálculo:

1. **simbólico pequeño:** reconstruir coeficientes exactos para corpus y
   demostrar identidades;
2. **evaluación multi-punto:** calcular el invariante en puntos identificados de
   uno o varios campos, con batch/SIMD y assurance explícito;
3. **Krylov/black-box:** determinante, polinomio mínimo y bilinear forms sin
   materializar siempre la inversa densa.

F251, Goldilocks y GF(2^256) forman baselines distintos. Múltiples puntos o
campos reducen colisiones observadas; una igualdad continúa siendo evidencia.
Los puntos fijos pertenecen a `GraphAnalysisProfileId`. Si una matriz es
singular en un punto:

- se registra el evento como información;
- se prueba el siguiente punto del bundle;
- nunca se divide por cero ni se publica una contracción parcial.

Para polinomios de grado acotado, suficientes evaluaciones distintas permiten
interpolación exacta sobre elementos codificados. Para el caso multivariante se
debe acotar grado y soporte; expandir sin cota causaría explosión combinatoria.

## Encaje diferencial con Microcanon

RGI no decide isomorfismo. Sus cuatro usos dentro del core son:

1. **rechazo:** una diferencia de un invariante correctamente construido prueba
   que dos grafos no son isomorfos;
2. **refinamiento:** features libres por vértice, como diagonales de resolvente,
   proponen colores equivariantes que se confirman o usan como refinador seguro;
3. **routing:** la degeneración por canal decide si probar RG2, 2-WL localizado
   o entrar directamente al IR;
4. **node invariant:** tras individualizar, el operador coloreado produce una
   traza más fuerte para ordenar/podar ramas bajo las reglas exactas del core.

La conexión más prometedora es incremental. Individualizar `v` puede modelarse
como una perturbación diagonal de rango uno:

```text
M' = M + alpha e_v e_v^T
det(M') = det(M) * (1 + alpha R[v,v])
R' = R - alpha (R e_v)(e_v^T R) / (1 + alpha R[v,v])
```

Cuando el denominador no es cero, Sherman–Morrison permite actualizar el
determinante y la resolvente de una rama sin invertir desde cero. Para varios
vértices se estudiará Woodbury. Las actualizaciones se verifican contra
recomputación completa; los puntos singulares hacen fallback.

Esto puede convertir una firma global potente en parte del planner de búsqueda,
aprovechando directamente campos, batch y workspaces del proyecto. Los bytes
canónicos siguen siendo independientes de que RGI esté habilitado.

## Qué sería diferenciador y qué no

No sería novedoso por sí solo:

- usar el espectro de la adyacencia o Laplaciana;
- calcular un polinomio característico;
- añadir varias matrices y comparar sus espectros;
- llamar “función de Green” a una resolvente;
- representar coeficientes como un heatmap o QR visual.

Podría ser una contribución real, sujeta a revisión bibliográfica y resultados:

1. un operador relacional inyectivo respecto del schema que cubra dirección,
   roles, multiplicidad e hiperaristas;
2. una familia mínima de contracciones RG2/RG3 no reducibles a invariantes
   conocidos y con discriminación demostrada;
3. evaluación multi-campo SIMD con wire, identidad y assurance reproducibles;
4. actualizaciones rank-one integradas en un IR exacto, con poda demostrada;
5. caracterización experimental y, donde sea posible, teórica de su potencia
   frente a 1-WL, 2-WL, zeta, generalized spectra y pattern counts;
6. extracción interpretable de propiedades, no solo un digest.

No se usará “estado del arte”, “completo” o “nuevo” antes de búsqueda de
anterioridad, formulación matemática revisada y comparación reproducible.

## Programa experimental

### RGI.0 — reproducción y sandbox matemático

- reproducir `Theta` en Sage solo para comprender determinant/resolvent/
  contraction y sus optimizaciones;
- construir operadores para grafos simples, dirigidos y relacionales;
- demostrar por conjugación la invariancia de cada candidato;
- encontrar contracciones redundantes mediante álgebra simbólica.

### RGI.1 — baseline de invariantes conocidos y catálogo de loops

- adjacency/Laplacian/signless/distance spectra;
- generalized characteristic polynomial;
- Ihara y Bartholdi zeta;
- traces y walk counts;
- 1-WL, 2-WL localizado y homomorphism/pattern counts disponibles;
- catálogo generado L0–L3, con `PatternId`, forma canónica independiente,
  `beta`, treewidth/coste y plan de evaluación;
- comprobación empírica de que L0 induce las mismas clases que 1-WL en el
  universo exhaustivo disponible.

Sin esta baseline no se puede atribuir valor a RGI.

### RGI.2 — selección de base y búsqueda de contracciones

- enumerar patrones/diagramas cerrados pequeños con tipos;
- quotient por simetrías y equivalencias algebraicas;
- comparar conteo explícito y resolventes como función generadora de la misma
  familia;
- seleccionar una familia por discriminación, coste, independencia y ley de
  composición;
- separar corpus de discovery y holdout para no sobreajustar parámetros.

### RGI.3 — engine de campo

- evaluaciones estáticas multi-punto y multi-campo;
- determinante/inversa densa de referencia;
- block Wiedemann/Krylov o alternativas sparse tras verificar corrección;
- batch SIMD y workspaces persistentes;
- singularidad y fallback transaccionales.

### RGI.4 — integración IR

- features rooted para partición/refinamiento;
- actualizaciones Sherman–Morrison/Woodbury;
- node traces y child ordering;
- comparación on/off con mismos bytes canónicos;
- poda solo después de una demostración separada.

### RGI.5 — evaluación científica

- todos los grafos simples hasta los órdenes manejables;
- corpus n=8/n=9 reservado;
- cospectrales, strongly regular, CFI, Miyazaki y alta simetría;
- dirigidos, labels, multigrafos e hipergrafos;
- moléculas y redes con split de dominio;
- curvas de colisión, coste, memoria y mejora de nodos IR.

## Gates

1. invariancia demostrada y tests metamórficos para cada contracción;
2. cero discrepancias entre simbólico y evaluado donde la interpolación sea
   suficiente;
3. colisiones mínimas y familias degeneradas publicadas;
4. comparación obligatoria contra spectrum/zeta/WL/pattern baselines;
5. el corpus de selección y el holdout permanecen separados;
6. integración on/off no cambia ninguna forma exacta;
7. mejora medida en rechazo o nodos IR compensa memoria y álgebra lineal;
8. revisión bibliográfica de anterioridad antes de nombrar o publicar;
9. todo claim incluye schema, campos, puntos, corpus, hardware y versión.

## Resultado esperado

El mejor resultado plausible no es “un polinomio que resuelve GI”. Es una
jerarquía de invariantes relacionales, rápida para un orden fijo, algebraicamente
interpretable, acelerada por campos finitos y capaz de reducir de forma material
los casos que alcanzan el IR exacto. Si RG2/RG3 no supera las baselines, el track
se clasifica como experimental o se descarta sin comprometer Microcanon.

Si sí las supera y las actualizaciones rank-one reducen el árbol, el proyecto
tendría una aportación diferenciadora real: un puente entre firmas
homomórficas, Green functions/tensor contractions y canonización certificada.

## Fuentes iniciales

- E. Klarreich,
  [*A Powerful New ‘QR Code’ Untangles Math’s Knottiest Knots*](https://www.quantamagazine.org/a-powerful-new-qr-code-untangles-maths-knottiest-knots-20260422/),
  como origen divulgativo de la analogía, no como fuente matemática.
- D. Bar-Natan y R. van der Veen,
  [*A Fast, Strong, Topologically Meaningful and Fun Knot Invariant*](https://arxiv.org/abs/2509.18456).
- J. K. Gamble et al.,
  [*Two-particle quantum walks applied to the graph isomorphism problem*](https://arxiv.org/abs/1002.3003).
- J. Smith,
  [*Cellular Algebras and Graph Invariants Based on Quantum Walks*](https://arxiv.org/abs/1103.0262).
- K. Rudinger et al.,
  [*Non-interacting multi-particle quantum random walks applied to the graph isomorphism problem for strongly regular graphs*](https://arxiv.org/abs/1206.2999).
- C. Durfee y K. Martin,
  [*Distinguishing Graphs with Zeta Functions and Generalized Spectra*](https://arxiv.org/abs/1410.1610).
- H. Dell, M. Grohe y G. Rattan,
  [*Lovász Meets Weisfeiler and Leman*](https://doi.org/10.4230/LIPIcs.ICALP.2018.40).
- Y. Chen et al.,
  [*On Algorithms Based on Finitely Many Homomorphism Counts*](https://doi.org/10.4230/LIPIcs.MFCS.2022.32).
- B. D. McKay y A. Piperno,
  [*Practical Graph Isomorphism, II*](https://arxiv.org/abs/1301.1493).
- M. Grohe et al.,
  [*Lovasz Meets Weisfeiler and Leman*](https://doi.org/10.4230/LIPIcs.ICALP.2018.40).
- La revisión sistemática de generalized graph spectra, quantum walks
  multipartícula, coherent/cellular algebras y tensor/homomorphism invariants
  sigue siendo un entregable previo a cualquier claim de anterioridad.
