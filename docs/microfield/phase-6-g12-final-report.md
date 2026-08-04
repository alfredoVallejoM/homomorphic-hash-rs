# Informe F6.G12 — comparación pareada exacta y loops largos

Fecha: 3 de agosto de 2026.

Estado: implementación y gates locales completados.

## Resultado ejecutivo

G12 elimina la necesidad de canonizar dos grafos completos para decidir una
comparación ordinaria. `Microcanon::compare` usa ahora un flujo exacto pareado:

1. metadatos y perfiles exactos baratos;
2. perfil bloque–corte del soporte relacional;
3. codificación exacta especializada de árboles y bosques;
4. refinamiento conjunto con colores comparables;
5. búsqueda fail-first de candidatos compatible con dirección, relación, rol y
   multiplicidad;
6. `VerifiedGraphMapping` antes de publicar `Isomorphic`.

Una diferencia de campo puede anticipar `Different` mediante
`compare_with_field_profile`, pero la firma se calcula dentro de la llamada. El
caller no puede asociar accidentalmente evidencia de otro grafo. Una igualdad
finita nunca autoriza `Isomorphic`.

G11 se amplía a la vez con `RelationalClosedWalkProfile`: consulta
`trace(A^k)` para longitudes positivas `u64`. Calcula como máximo `2n + 1`
términos, recupera una recurrencia por Berlekamp–Massey y evalúa índices lejanos
por exponenciación en el anillo cociente. Son closed walks exactos sobre el
campo, no conteos de ciclos simples.

## Campos finitos

El núcleo exacto sigue sin ser genérico sobre un campo. Los campos solo
participan en perfiles auxiliares.

- cualquier campo primo o extensión binaria validada puede construirse en
  runtime;
- `DynamicGraphFieldProfile` aplica la misma política por característica que
  el tipo generado;
- `export_manifest` y la factory convierten el contexto validado en un tipo
  estático monomorfizado para el hot path;
- el fixture externo GF(2^9) prueba identidad y comportamiento iguales entre
  el contexto runtime y el tipo generado;
- cada canal declara `Field`, `Pow` o `Invert` únicamente cuando los necesita.

No se afirma todavía soporte para extensiones generales GF(p^m), con `p` impar
y `m > 1`. El soporte constructor mantenido cubre GF(p) y GF(2^m).

## Loops largos

`LoopPatternCatalog` conserva deliberadamente el contrato inducido L0–L3,
orden máximo cuatro. No se amplía el factorial.

`ClosedWalkQueryPlan` separa ahora la longitud de un recorrido cerrado del
rango ciclomático de un patrón. Sus consultas:

- están ordenadas, deduplicadas e identificadas;
- admiten hasta 1.024 longitudes positivas;
- usan exponentes `u64`, incluido `10^12` en los tests;
- poseen preflight y `SkippedBudget` atómico;
- suman exactamente bajo unión disjunta;
- tienen wire `MFCW` ligado a campo, encoder, lanes y plan;
- coinciden término a término con `RelationalMatrixProfile` en las longitudes
  calculadas por ambos algoritmos.

El plan puede ejecutarse sobre dos operadores:

- `Adjacency`, con labels de vértice en diagonal y relaciones dirigidas;
- `NonBacktracking`, sobre estados de incidencia, excluyendo la reversión
  inmediata de una arista y conservando relación, rol y multiplicidad.

La propagación de semillas usa el operador disperso, aunque conserva una
matriz de potencias para obtener trazas. Su coste de preparación es adecuado
para grafos pequeños/medios y está siempre acotado por presupuesto. No se vende
como contador eficiente de todos los ciclos simples, problema combinatorio
distinto.

## Comparación pareada

### Prefiltros exactos

Se comparan kinds, labels, self-loops, bloque–corte, incidencias y
multiplicidades. Los witnesses distinguen:

- metadata básica;
- descriptores de vértice;
- partición estable;
- perfil bloque–corte;
- bosques;
- canal finito calculado internamente;
- agotamiento exacto del espacio de candidatos.

### Árboles y bosques

El soporte forestal usa interning conjunto y exacto de subárboles. No emplea
hashes ni bytes recursivamente anidados. El recorrido y el block-cut de bosques
son iterativos, por lo que un camino de 4.096 vértices completa en tests sin
recursión ni crecimiento cuadrático de estado.

Centros dobles, componentes repetidas, aristas dirigidas, roles, labels,
multiplicidad y self-loops forman parte de las claves exactas. Los empates entre
subárboles idénticos pueden resolverse en cualquier orden porque el mapping
completo se verifica después.

### Bloque–corte y matcher general

Tarjan iterativo construye bloques biconexos y articulaciones. Sus tamaños y
perfiles se reinyectan en la partición conjunta. La búsqueda general:

- mantiene dominios compatibles por celda;
- elige el vértice con menos candidatos;
- comprueba arcos en ambos sentidos frente al mapping parcial;
- respeta nodos, profundidad, tiempo, celdas retenidas y bytes rastreados;
- devuelve `CandidateSpaceExhausted` solo después de explorar todos los
  mappings compatibles;
- devuelve `Inconclusive` si termina cualquier presupuesto.

## Evidencia determinista

La campaña `g12-v1` contiene:

- 1024 pares diferenciales frente a formas canónicas independientes;
- 729 de 1.033 comparaciones totales terminan en el prefiltro exacto tras
  incorporar los descriptores conjuntos de grado;
- seis relabelings verificados, hasta un bosque de 4.096 vértices;
- cero resultados inconclusos inesperados;
- C6 frente a 2C3: rechazo exacto por bloque–corte;
- Shrikhande frente a rook 4×4: espacio agotado en 304 asignaciones;
- CFI(K4) par frente a twist: espacio agotado en 6.976 asignaciones.

El resultado reproducible está en
[`g12-v1.json`](../../validation/f6/results/g12-v1.json).
Su SHA-256 es
`fc1a3dd48de4efca86ca71dcd0c7198c4ac3465e5d4713deb1f4bb9991c24648`;
una segunda ejecución completa produjo exactamente los mismos bytes.

## Rendimiento local

Criterion `--quick`, release LTO, Intel Core i7-13700HX:

| Comparación de caminos | Pareado G12 | Dos canonizaciones | Mejora |
|---|---:|---:|---:|
| n=128 | 428,30 µs | 3,7805 ms | 8,83× |
| n=1.024 | 3,9349 ms | 285,06 ms | 72,44× |
| n=4.096 | 16,267 ms | no ejecutado | — |

El coste pareado observado es aproximadamente lineal en esta familia.

Closed-walk Goldilocks K3, consultas 16, 64 y 10^12:

| Orden | Tiempo |
|---:|---:|
| 8 | 88,294 µs |
| 16 | 435,87 µs |
| 32 | 2,5549 ms |

Para el operador no-backtracking en los mismos ciclos:

| Orden | Tiempo |
|---:|---:|
| 8 | 153,97 µs |
| 16 | 805,13 µs |
| 32 | 5,3694 ms |

Estas cifras son locales y no sustituyen calibración multi-CPU.

## Gates superados

- todo `Isomorphic` incluye mapping revalidado;
- fingerprints solo producen negativas;
- 1024 comparaciones coinciden con el oráculo canónico;
- invariancia sobre 128 multigrafos relacionales deterministas;
- CFI y strongly regular se deciden exactamente;
- todos los límites fallan cerrados tanto en búsqueda general como en la ruta
  especializada de bosques;
- bosque de 4.096 vértices sin recursión;
- campo externo generado y contexto runtime comparten `FieldId` y policy;
- closed walks coinciden con trazas densas y admiten longitudes `u64`;
- non-backtracking anula árboles y detecta ciclos sin rebotes inmediatos;
- workspace completo, Clippy y benchmarks compilan sin warnings.

## Límite final

Canonización y comparación exacta tienen peor caso exponencial en grafos
generales. G12 reduce el coste común y conserva presupuestos explícitos; no
afirma resolver en tiempo polinómico el isomorfismo general ni contar todos los
ciclos simples. En el momento de este cierre G13–G15 quedaban como líneas de
evolución; G13/G14 fueron implementados posteriormente y G15 dispone ahora de
un plan separado de preparación interna.
