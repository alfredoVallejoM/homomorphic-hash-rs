# ADR 0031 — Núcleo de canonización certificado e invariantes auxiliares

- Estado: aceptado; G8–G10 implementados
- Fecha: 3 de agosto de 2026
- Reabre: F6.G6–G7 como baseline experimental, no como cierre público

## Contexto

La implementación actual mezcla tres conceptos que deben tener contratos
independientes:

1. firmas finitas y digests útiles para filtrar candidatos;
2. refinamiento local y motivos usados para reducir ambigüedad;
3. canonización exacta mediante individualización–refinamiento.

`FastGraphLabeler::canonicalize_exact` depende hoy de un perfil de campo, usa
una ruta discreta ligada a `GraphSignatureId`, crea claves byte dinámicas en
cada pasada y realiza una búsqueda exhaustiva sin poda por automorfismos,
trazas de nodo o cotas de prefijo. La ruta es correcta cuando completa, pero no
constituye todavía un canonizador industrial ni una identidad persistente
independiente del acelerador.

La propuesta inspirada por el invariante de nudos
`Theta = (Delta, theta)` aporta una idea de diseño valiosa: enriquecer un
invariante rápido de primer orden con interacciones de segundo orden y conservar
un objeto polinómico con más información que una evaluación escalar. No permite
trasladar directamente las fórmulas de nudos a grafos. Los nudos se identifican
bajo movimientos de Reidemeister y los grafos bajo permutaciones de vértices;
son problemas de equivalencia distintos. Además, ya existe trabajo previo
sobre Green functions y caminatas de dos partículas para isomorfismo de
grafos. Cualquier variante propia debe compararse con ese estado previo y con
2-WL antes de reclamar novedad.

## Decisión

Se construirá `Microcanon v1` como un núcleo exacto independiente de campos,
firmas y perfiles de ejecución.

Su definición semántica será:

```text
Canon(G) = min_lex { Encode(G^pi) | pi es una permutación admisible }
```

`Encode` será una serialización inyectiva y versionada del modelo relacional
normalizado. Una permutación admisible preserva el schema, el tipo y la
semántica exacta de vértices, relaciones, roles, dirección, bucles,
multiplicidades e hiperaristas.

La implementación usará un árbol de individualización–refinamiento:

1. partición inicial exacta;
2. refinamiento relacional equitativo hasta punto fijo;
3. descomposiciones demostrablemente canónicas;
4. selección invariante de una celda no unitaria;
5. individualización exhaustiva de sus candidatos;
6. poda únicamente mediante cotas y automorfismos demostrados;
7. mínimo lexicográfico entre todas las hojas no podadas.

WL no define el resultado. 1-WL será el refinador equitativo inicial y 2-WL
localizado podrá reforzar celdas ambiguas. Si cualquiera de ellos no separa un
grafo, la búsqueda continúa. Si un presupuesto impide completar el árbol, el
resultado será `BudgetExhausted`/`Inconclusive`; nunca se publicará el mejor
candidato parcial.

Las firmas algebraicas y los invariantes de interacción podrán:

- rechazar pares con evidencia distinta;
- proponer agrupaciones que se confirman con claves exactas;
- ordenar trabajo y seleccionar refinadores;
- producir trazas, momentos y propiedades componibles;
- acelerar la búsqueda sin modificar los bytes canónicos.

No podrán:

- podar una rama por igualdad de firma;
- confirmar isomorfismo por igualdad;
- cambiar el orden canónico según campo, número de lanes o CPU;
- aparecer dentro de `CanonicalGraphEncodingId`.

Toda poda tendrá una obligación de prueba concreta. Las optimizaciones se
mantendrán desactivables y se contrastarán con un canonizador de referencia
por fuerza bruta en órdenes pequeños.

## Fachadas resultantes

```rust
pub enum GraphComparison {
    Different { witness: DifferenceWitness },
    Isomorphic { mapping: VerifiedGraphMapping },
    Inconclusive { report: ComparisonReport },
}

pub enum CanonicalOutcome {
    Exact { form: CanonicalGraphForm, report: CanonicalReport },
    BudgetExhausted { report: CanonicalReport },
}
```

Una correspondencia solo se publicará después de que un verificador lineal
compruebe de nuevo todos los vértices e incidencias exactas. El digest de una
forma exacta será una clave de índice; la igualdad autoritativa comparará los
bytes completos.

## Track de investigación de loops e interacción

Se investigará un `RelationalInteractionInvariant` separado del canonizador.
Su semántica combinatoria de referencia serán conteos de homomorfismos de un
catálogo generado de patrones relacionales graduados por número de loops y
treewidth. L0 (árboles) conecta con 1-WL; L1, L2 y L3 añaden patrones
unicíclicos, theta/barbell y superiores. Un orden fijo sigue siendo incompleto.

La formulación matricial estudiará un operador tipado de una partícula `A_G` y
un operador de dos partículas con interacción:

```text
H2(G) = A_G tensor I + I tensor A_G + J_G
```

`J_G` codifica coincidencia, adyacencia y descriptores relacionales de un par.
Bajo una renumeración `P`, el operador se transforma por semejanza mediante
`P tensor P`; por ello trazas, polinomios característicos y contracciones
simétricas son candidatos a invariantes.

Se ensayarán conteos explícitos, representaciones evaluadas en varios campos y
puntos, resolventes/contracciones, expansiones de baja longitud y formas
dispersas. El track solo se promocionará si:

1. posee una demostración ejecutable de invariancia por renumeración;
2. separa pares que sobreviven a los canales actuales;
3. aporta información no redundante frente a 2-WL, espectro, Ihara/no-backtrack
   y motivos tipados;
4. su coste y memoria son predecibles;
5. sus claims se limitan a la evidencia medida.

El nombre `Theta` no se reutilizará para evitar sugerir equivalencia matemática
con el invariante de nudos.

## Consecuencias

- La corrección deja de depender de la calidad de una firma o de WL.
- Los bytes canónicos serán estables entre campos y estrategias.
- Los grafos fáciles conservarán una ruta casi lineal; los difíciles pagarán
  refinamiento de pares o búsqueda exacta solo dentro de presupuesto.
- Las firmas mantienen valor comercial y científico como índices,
  resúmenes componibles y descriptores estructurales, sin venderse como pruebas.
- El canonizador actual se conservará temporalmente como baseline diferencial y
  adapter de compatibilidad; no recibirá nuevas heurísticas.
- No se reclamará un algoritmo polinómico general ni una innovación científica
  hasta completar revisión de literatura, baselines y contraejemplos.

## Referencias de diseño

La ejecución de esta decisión se detalla en el
[plan F6.G8–G15](../phase-6-canonization-v2-plan.md). La analogía de segundo
orden, sus fórmulas candidatas y sus gates se aíslan en la
[investigación de jerarquía relacional de Green](../relational-green-invariant-research.md).

- McKay y Piperno, [Practical Graph Isomorphism, II](https://arxiv.org/abs/1301.1493).
- Bar-Natan y van der Veen,
  [A Fast, Strong, Topologically Meaningful and Fun Knot Invariant](https://arxiv.org/abs/2509.18456).
- Gamble et al.,
  [Two-particle quantum walks applied to the graph isomorphism problem](https://arxiv.org/abs/1002.3003).
- Smith,
  [Cellular Algebras and Graph Invariants Based on Quantum Walks](https://arxiv.org/abs/1103.0262).
- Dell, Grohe y Rattan,
  [Lovasz Meets Weisfeiler and Leman](https://doi.org/10.4230/LIPIcs.ICALP.2018.40).
