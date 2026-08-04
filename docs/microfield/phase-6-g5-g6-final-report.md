# Informe final F6.G5–G6 — adversariales y canonización exacta acotada

Fecha: 2 de agosto de 2026.

## Resultado

> Nota posterior: este cierre fue reabierto por F6.G7. El diagnóstico de este
> documento sigue siendo válido, pero la firma local dejó de ser la fachada
> recomendada de discriminación. Véase
> [phase-6-g7-final-report.md](phase-6-g7-final-report.md).

F6.G5–G6 quedaron cerrados. El sistema conserva como primitiva el
etiquetado algebraico lineal por ronda, pero ahora puede medir por qué una
firma pierde discriminación y, cuando el consumidor lo solicita, escalar a una
canonización exacta con límites duros.

No se ha convertido ninguna firma finita en una prueba. La taxonomía pública es
deliberada:

```text
Different          => algún canal identificado difiere
Indistinguishable  => todos los canales coinciden; isomorfismo no decidido
Exact              => árbol completo y forma canónica exacta
BudgetExhausted    => no hay afirmación exacta ni candidato publicado
```

## F6.G5: qué degeneración se mide

`FastGraphLabeler::diagnose_degeneracy` ejecuta el perfil solicitado y lo
contrasta con un refinamiento 1-WL independiente de la aritmética. El oráculo
local usa bytes exactos para:

- tipo y etiqueta del vértice;
- dirección de la incidencia;
- relación y rol;
- multiplicidad `u64`;
- multiconjunto ordenado de colores vecinos.

Si `T` es el número de pasadas hasta estabilizar y `d` la mayor fila CSR, el
coste es `O(T (V + I log d))`, con `T ≤ V`. El corpus regular medido estabiliza
enseguida; esa observación no se convierte en una cota general optimista.

`GraphDegeneracyReport` publica los números de clases rápida y exacta, tamaños
de sus mayores clases, vértices ambiguos y clases/vértices afectados por
aliasing. Esto separa dos fenómenos:

| Fenómeno | Diagnóstico | ¿Ayudan más lanes/campos? | Promoción con garantía |
|---|---|---|---|
| colisión modular o de encoding | una clase rápida contiene varias clases exactas | pueden reducirla | canonización exacta |
| indistinguibilidad local | la 1-WL exacta conserva clases no unitarias | no necesariamente | individualización–refinamiento exacto |
| alta regularidad | 75 % ambiguo y una clase ≥ 25 % de `V`, con `V ≥ 4` | no si los descriptores son iguales | búsqueda exacta presupuestada |

La marca de alta regularidad es una política de routing v1, no una definición
universal de grafo regular. Sus componentes numéricos permanecen accesibles
para que una aplicación aplique umbrales propios.

### No existe un límite de tamaño que cure el problema

El corpus contiene `C_n` y la unión disjunta `C_a ⊔ C_b`, con
`a + b = n`, `a,b ≥ 3`. Todos los vértices son anónimos, de grado dos y ven el
mismo descriptor exacto en cada ronda. Por inducción, una recurrencia local
invariante les asigna siempre la misma etiqueta.

La primera colisión no isomorfa aparece ya con seis vértices:

```text
C6  frente a  C3 ⊔ C3
```

La suite la reproduce con F251, GF(2²⁵⁶), varias lanes, 1–43 rondas y el canal
híbrido SHA-256. SHA-256 no falla criptográficamente: recibe el mismo
descriptor refinado y, por tanto, resume la misma información. SageMath 10.7
confirma de forma independiente 35 pares no isomorfos de esta familia para
`6 ≤ V ≤ 40`, mientras una 1-WL exacta conserva una sola clase.

El corpus incluye además el par fuertemente regular Shrikhande/torres 4×4.
Ambos tienen parámetros `(16,6,2,2)`, una sola clase 1-WL y la misma evidencia
local, pero Sage confirma que no son isomorfos. El detector marca los 16
vértices ambiguos y recomienda búsqueda exacta; un presupuesto insuficiente
falla cerrado.

El tamaño puede aumentar la probabilidad de colisión en un codominio finito,
pero no hay un `V` a partir del cual dos firmas iguales pasen a significar
isomorfismo. Tampoco hay un número de rondas universal que rompa regularidad.

## Evidencia multi-campo identificada

`MultiFieldGraphEvidenceBuilder` acepta firmas estáticas heterogéneas sin
introducir dispatch dinámico en su cálculo. Antes de publicar:

1. comprueba que los contadores exactos de grafo coinciden;
2. rechaza canales duplicados;
3. ordena los canales por `GraphSignatureId`;
4. deriva un `GraphEvidenceProfileId` domain-separated de todo el conjunto.

Por tanto, campo, encoder, lanes, parámetros experimentales y rondas forman
parte de la identidad. El orden de inserción no cambia el bundle. Comparar
perfiles diferentes falla cerrado. Comparar perfiles compatibles solo produce
`Different` o `Indistinguishable`; una coincidencia multi-campo no prueba
isomorfismo y el corpus regular demuestra por qué.

## F6.G6: canonización exacta opt-in

`canonicalize_exact(graph, CanonicalSearchBudget)` aplica esta secuencia:

1. ejecuta el análisis rápido y el diagnóstico exacto;
2. si las etiquetas rápidas son unitarias, reutiliza su orden invariante sin
   consumir nodos de búsqueda;
3. en caso contrario, estabiliza colores exactos;
4. selecciona de forma invariante la menor clase no unitaria, con desempate por
   color canónico;
5. individualiza cada candidato y vuelve a refinar;
6. recorre todo el árbol mediante DFS iterativo;
7. serializa cada hoja discreta y conserva el mínimo lexicográfico;
8. publica la forma solo después de agotar completamente el árbol.

No hay poda probabilística, índice original mezclado en colores, recursión de
pila ni aleatoriedad de proceso. La ausencia de poda hace que el número total
de nodos no dependa del orden de visita y simplifica el argumento de
corrección. El coste de peor caso continúa siendo exponencial, como debe
declararse para una canonización general.

`CanonicalSearchBudget` controla:

- `max_search_nodes`;
- `max_retained_state_cells`, con 16 Mi celdas `usize` como valor conservador
  del constructor.

`CanonicalSearchReport` conserva ruta, nodos, hojas, individualizaciones,
pasadas exactas, profundidad, pico retenido y límite agotado. Si falta
presupuesto, `BudgetExhausted` no contiene `DiscreteCanonicalForm`.

La serialización exacta conserva el envelope versionado previo, incluyendo
`GraphSignatureId`. Las formas se comparan bajo el mismo perfil identificado.
El cuerpo es inyectivo para el grafo normalizado ordenado: incluye todos los
vértices y todas las incidencias con sus metadatos exactos.

## Evidencia de corrección

La nueva suite `tests/graph_canonical.rs` contiene doce contratos agrupados en
once tests de integración:

- colisión multi-campo e híbrida `C6`/`2C3`, separada por canonización exacta;
- barrido de regularidad entre 6 y 40 vértices y distintos números de rondas;
- par fuertemente regular Shrikhande/torres 4×4 y escalado fail-closed;
- 64 renumeraciones de un ciclo simétrico con bytes y nodos idénticos;
- agotamiento exacto en `required_nodes - 1` y éxito en `required_nodes`;
- agotamiento independiente por estado retenido;
- colisión forzada por palomar en F251, diagnosticada como aliasing y no como
  regularidad;
- ruta sin búsqueda cuando la partición rápida es discreta;
- 128 normalizaciones con renumeración, orden de aristas aleatorio y
  multiplicidades fragmentadas;
- preservación de dirección, rol, multiplicidad y componentes;
- oráculo exhaustivo independiente sobre los 1.088 grafos simples etiquetados
  de cuatro y cinco vértices, agrupados correctamente en 11 y 34 clases de
  isomorfismo;
- identidad, orden independiente, incompatibilidades y semántica
  `Indistinguishable` del bundle multi-campo.

El script `tools/sage/verify_graph_degeneracy.sage`, ejecutado en
`laboratorio_np`, produjo:

```json
{"ok":true,"oracle":"SageMath Graph.is_isomorphic + exact Python 1-WL","sage_version":"10.7","non_isomorphic_regular_pairs":35,"non_isomorphic_strongly_regular_pairs":1,"minimum_collision_vertices":6}
```

## Coste observado

Medición Criterion `--quick`, release LTO, Intel i7-13700HX, 2 de agosto de
2026. Son cifras locales y no thresholds de API.

| Diagnóstico F251 K=3/R=4 sobre ciclo | Tiempo | Incidencias/s |
|---|---:|---:|
| 1.024 vértices | 750–755 µs | 2,71–2,73 M |
| 16.384 vértices | 13,80–14,03 ms | 2,34–2,37 M |
| 131.072 vértices | 113,38–115,46 ms | 2,27–2,31 M |

| Canonización exacta de ciclo homogéneo | Tiempo |
|---|---:|
| 6 vértices | 186–192 µs |
| 8 vértices | 363–378 µs |
| 10 vértices | 1,06–1,10 ms |
| 12 vértices | 1,60–1,61 ms |

El diagnóstico es deliberadamente más caro que `analyze`: construye y ordena
descriptores exactos. No se ha insertado en el hot path. Los tiempos pequeños
del corpus de ciclos no generalizan a familias de búsqueda difícil; el
presupuesto, no una extrapolación, gobierna la estabilidad.

## Cierre y límites restantes

F6.G5 y F6.G6 cerraron su corte junto con la migración legado, las firmas
genéricas, el motor de grafos, SIMD/parallel y actualización incremental ya
entregados. Permanecen fuera del contrato:

- resistencia criptográfica o adversarial de las firmas;
- decisión de isomorfismo por igualdad de firma, SHA o bundle;
- canonización exacta incremental;
- garantía polinómica para la búsqueda exacta;
- selección automática de presupuesto para una aplicación desconocida.

F6.G7 añadió después la información global rápida y el corpus externo sin
invalidar estos contratos de diagnóstico y exactitud.
