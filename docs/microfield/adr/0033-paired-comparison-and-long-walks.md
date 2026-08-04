# ADR 0033 — comparación pareada exacta y closed walks largos

Fecha: 3 de agosto de 2026.

Estado: aceptado.

## Contexto

El comparador G10 canonizaba ambos grafos completos. Era correcto, pero no
aprovechaba que una comparación solo necesita encontrar un mapping o demostrar
que no existe. A la vez, ampliar el catálogo inducido de orden cuatro para
representar ciclos largos habría introducido coste factorial.

## Decisión

1. `Microcanon::compare` usa un matcher exacto pareado separado de la emisión
   de bytes canónicos.
2. El matcher comparte refinamiento, dominios y presupuesto entre ambos grafos.
3. Bosques usan interning estructural exacto y no recursivo.
4. Grafos con articulaciones incorporan un árbol bloque–corte calculado con
   Tarjan iterativo.
5. Todo mapping candidato se verifica contra el modelo relacional completo.
6. Los perfiles finitos se calculan internamente; una diferencia rechaza y una
   igualdad continúa por la ruta exacta.
7. La longitud de closed walks se modela en `ClosedWalkQueryPlan`, separada del
   `cycle_rank` de patrones inducidos.
8. Longitudes lejanas usan una recurrencia certificada por suficientes términos
   de traza y Cayley–Hamilton, no enumeración de subgrafos.
9. El operador no-backtracking vive en estados de incidencia y excluye la
   reversión inmediata, sin expandir multiplicidades en aristas físicas.
10. Presupuesto agotado siempre produce `Inconclusive`.

## Alternativas rechazadas

- confiar en bundles externos no ligados exactamente al grafo;
- elevar `LoopPatternCatalog` por encima de orden cuatro;
- usar digests SHA como autoridad de subárboles;
- devolver isomorfismo por igualdad de varios campos;
- mantener dos canonizaciones como única ruta de comparación;
- DFS recursivo para árboles o Tarjan, por riesgo de stack en grafos grandes.

## Consecuencias

- los bytes canónicos no cambian;
- comparación y canonización pueden evolucionar y medirse por separado;
- G11 captura longitudes de closed walks `u64`, pero no promete ciclos simples;
- campos primos y binarios generados siguen siendo aceleradores monomorfizados;
- el peor caso general permanece exponencial y está controlado por presupuesto.
