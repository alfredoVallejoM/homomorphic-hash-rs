# Informe de implementación y primera evidencia — F6.V1–F6.V6

Fecha: 3 de agosto de 2026.

## Resultado ejecutivo

F6.V1–F6.V6 ya disponen de implementación reproducible. Se ha añadido un
laboratorio Rust no publicable, manifiesto congelado, corpus matemático de
Sage/nauty, oráculos adversariales independientes, JSON/CSV deterministas,
captura separada de rendimiento y gates CI x86-64/AArch64.

La primera ejecución completa cambia la lectura del producto de forma útil:

- las leyes homomórficas funcionan, pero una sola evaluación F251 colisiona
  pronto en dominios pequeños;
- la secuencia bidireccional y el multiconjunto multievaluado reducen mucho esa
  degeneración, sin convertirse en pruebas de igualdad;
- la reconciliación acotada funciona de extremo a extremo con distancia
  desconocida en el universo validado;
- en grafos simples de ocho vértices, SHA híbrido y un segundo campo no corrigen
  la indistinguibilidad combinatoria de 1-WL;
- los motivos adaptativos reducen los 454 grafos ambiguos del perfil rápido a
  46, pero todavía dejan 22 buckets;
- la canonización exacta separa los contraejemplos comprobados, incluidos CFI;
- la actualización incremental deja de ser atractiva gradualmente: en el caso
  medido recorre 1 %, 10 %, 80 %, 96,875 % y 100 % del trabajo completo para
  ráfagas de 1, 10, 100, 250 y 500 vértices sobre 1.000.

Por tanto, la arquitectura escalonada queda respaldada: firma rápida para
filtrar, perfil adaptativo para reducir candidatos y canonización exacta como
única confirmación general. No queda respaldado presentar SHA o multi-campo
como remedio universal de la regularidad.

## Infraestructura F6.V0 habilitante

El crate privado `microfield-validation-lab` contiene:

- parser estricto graph6;
- enumeradores exhaustivos de firmas;
- catálogo reproducible de colisiones mínimas;
- reconciliación racional por polinomios característicos;
- campañas de grafos y verticales aplicadas;
- medición host-specific con inventario de arquitectura, Rust y features ISA;
- CLI `f6-validation semantic|performance`;
- serialización JSON y tabla CSV estable.

El manifiesto `validation/f6/manifest.json` fija seed, alfabetos, longitudes,
universo de reconciliación, rondas, presupuestos, warm-up, repeticiones y
tamaños hasta un millón de vértices. Los tiempos no son golden tests; los
resultados semánticos sí deben regenerarse con diff vacío.

## F6.V1 — leyes y metamorfismo

La campaña F251 enumera 5.461 palabras de alfabeto 4 hasta longitud 6 y ejecuta
145.636 ecuaciones de partición/merge entre suma, secuencia, bidireccional y
multiconjunto. Una segunda suite exhaustiva cubre suma, secuencia y
multiconjunto sobre GF(2²⁵⁶) hasta longitud 4.

Además se mantienen los tests anteriores de:

- identidad nominal y separación de contexto;
- wire canónico y rechazo de estados imposibles;
- atomicidad ante error;
- equivalencia estática/dinámica;
- tracking exacto;
- batch/ISA frente a escalar en Microfield.

El control negativo residual confirma de forma ejecutable que un término falso
puede producir un residuo que recompone su propia ecuación. El residual no es
una prueba de pertenencia.

## F6.V2 — colisiones mínimas observadas

Campaña congelada: alfabeto `{0,1,2,3}`, longitud/cardinalidad hasta 5. Las
entradas con la misma semántica normativa se deduplican antes de contar.

| Firma | Entradas | Salidas | Buckets | Entradas colisionadas | Primer tamaño |
|---|---:|---:|---:|---:|---:|
| aditiva F251 | 126 | 51 | 30 | 105 | 2 |
| secuencia F251 | 1.365 | 463 | 334 | 1.236 | 3 |
| bidireccional F251 | 1.365 | 1.363 | 2 | 4 | 5 |
| multiconjunto K=1 | 126 | 125 | 1 | 2 | 5 |
| multiconjunto K=2 | 126 | 126 | 0 | 0 | — |
| multiconjunto K=4 | 126 | 126 | 0 | 0 | — |

Contraejemplos mínimos conservados:

- aditiva: `[0,2]` y `[1,1]`;
- secuencia: `[0,0,2]` y `[2,1,0]`;
- bidireccional: `[0,3,2,3,0]` y `[3,0,0,0,3]`;
- multiconjunto K=1: `[0,0,0,0,1]` y `[2,3,3,3,3]`.

La ausencia de colisión K=2/K=4 solo vale para este dominio. Los puntos son
fijos y no se atribuye una cota Schwartz–Zippel.

## F6.V3 — aplicaciones de firmas

### Agregación y secuencias

La composición distribuida queda clasificada como `ValidatedPrimitive`: toda
partición normativa coincide con la ingestión directa y el merge no relee el
prefijo. Sigue necesitando confirmación exacta cuando se usa para identidad.

La medición local ilustrativa sobre Intel i7-13700HX obtuvo, para 65.536 bytes:

- recomputación secuencial F251: mediana 328,938 µs;
- merge de dos firmas ya calculadas: mediana 0,279 µs;
- SHA-256 de los bytes: mediana 31,544 µs.

El merge mide el caso de uso correcto —particiones precomputadas— y no incluye
el coste previo de firmar cada partición. Los datos multi-CPU los publicará CI
como artefactos, no como constantes de documentación.

### Reconciliación acotada

Se implementó el paso que faltaba entre “producto evaluado” y “recuperar la
diferencia”:

1. ambos conjuntos evalúan su polinomio característico en `d` puntos;
2. el receptor forma el cociente de evaluaciones;
3. prueba grados compatibles con la diferencia de cardinalidades y el límite;
4. resuelve el sistema lineal de los polinomios numerador/denominador;
5. factoriza en el universo declarado;
6. reconstruye el conjunto y verifica todas las evaluaciones transmitidas.

Pasaron 63.232 pares exhaustivos de subconjuntos del universo de ocho elementos
con diferencia simétrica `≤ 6`. El decoder no recibe la distancia exacta. Un
caso de siete diferencias se rechaza fail-closed.

Clasificación: `ValidatedPrimitive`. Antes de API pública faltan factorización
escalable, multiplicidades, negociación de parámetros, límites de memoria y
comparación con un sketch de reconciliación mantenido.

## F6.V4 — grafos simples y adversariales

SageMath 10.7 generó con `graphs.nauty_geng("8")` exactamente 12.346
representantes no isomorfos. El corpus graph6 pesa 86.422 bytes y tiene SHA-256
`546a249902101c97d3aa590f93e53366854bd0a6f405aa59bdb32d25c57f845a`.

| Escalón | Salidas distintas | Buckets | Grafos | Pares candidatos | Máximo bucket |
|---|---:|---:|---:|---:|---:|
| F251 rápido v1 | 12.095 | 203 | 454 | 350 | 8 |
| F251 + SHA híbrido | 12.095 | 203 | 454 | 350 | 8 |
| perfil global v2 | 12.103 | 198 | 441 | 335 | 8 |
| v2 + motivos adaptativos | 12.322 | 22 | 46 | 27 | 4 |
| F251 + GF(2²⁵⁶) rápido | 12.095 | 203 | 454 | 350 | 8 |

Se comprobaron 128 renumeraciones muestreadas sin falsos negativos de
invariancia. El primer par v1 conservado es `G?BDB?` / ``G?`CQG``: SHA y el
segundo campo no lo separan; global v2, motivos y exacto sí.

Sage `Graph.is_isomorphic` certifica como no isomorfos y el exacto separa:

- `C6` frente a `2C3`;
- Shrikhande frente al grafo de torres 4×4;
- CFI(K4) par frente a una arista torcida, 40 vértices y 60 aristas.

Los tres colisionan en rápido e híbrido. El resultado confirma que cambiar de
campo no añade información cuando la recurrencia local ya produjo exactamente
los mismos descriptores.

## F6.V5 — verticales e incrementalidad

Se añadieron pilotos tipados de molécula, red dirigida, grafo de conocimiento e
hipergrafo. Los cuatro conservan el digest adaptativo bajo renumeración y
detectan la perturbación semántica elegida —orden de enlace, dirección, tipo de
relación o rol hiperarista—.

Esto valida el modelado, no todavía un producto de dominio. Los corpus externos
pinneados —MUTAG, SNAP email-Eu, diseasome XGI y Graph Atlas— se ejecutan
semanalmente y bajo `workflow_dispatch`; el fetch verifica SHA-256 y el segundo
pase se fuerza offline.

La curva incremental determinista sobre un ciclo etiquetado de 1.000 vértices y
ocho rondas es:

| Vértices editados | Vertex-rounds incrementales | Trabajo relativo |
|---:|---:|---:|
| 1 | 80 | 1 % |
| 10 | 800 | 10 % |
| 100 | 6.400 | 80 % |
| 250 | 7.750 | 96,875 % |
| 500 | 8.000 | 100 % |

Cada punto coincide byte a byte con recomputación completa. La política futura
debe poder abandonar incrementalidad antes de la saturación del cono.

## F6.V6 — rendimiento y reproducción

El runner separa recomputación, merge, SHA y grafo preparado. En la primera
máquina x86-64, el perfil preparado de ocho rondas dio aproximadamente:

| Vértices sparse | Mediana |
|---:|---:|
| 1.000 | 0,895 ms |
| 10.000 | 8,295 ms |
| 100.000 | 49,733 ms |
| 1.000.000 | 491,150 ms |

Son cifras preliminares de una ejecución, no un claim estable. El workflow
genera artefactos separados en `ubuntu-latest` x86-64 y
`ubuntu-24.04-arm`. Completar la matriz exigida de dos microarquitecturas Intel,
dos AMD y dos familias ARM continúa como requisito de publicación: el código y
la captura están completos, la evidencia de hardware no se inventa.

## Clasificación actual

| Capacidad | Clasificación | Decisión |
|---|---|---|
| suma/paridad | `ValidatedPrimitive` | mantener, documentar alta degeneración |
| secuencia Horner | `ValidatedPrimitive` | mantener para composición, nunca identidad sola |
| secuencia bidireccional | `Experimental` | prometedora; ampliar corpus/fields |
| multiconjunto K=1 | `ValidatedPrimitive` | mantener como componente barato |
| multievaluación K=2/K=4 | `Experimental` | medir coste/beneficio a mayor escala |
| residual | `Rejected` como pertenencia | conservar solo como ecuación algebraica |
| reconciliación acotada | `ValidatedPrimitive` | productizar decoder antes de estabilizar |
| grafo rápido v1 | `ValidatedPrimitive` | filtro lineal, no canonizador |
| SHA híbrido | `Rejected` como solución a regularidad | útil solo ante descriptores adicionales distintos |
| bundle multi-campo local | `Rejected` como solución a 1-WL | útil únicamente contra aliasing de campo |
| v2 adaptativo | `ValidatedPrimitive` | escalón recomendado, aún no exacto |
| canonización exacta | `ValidatedPrimitive` | única confirmación general, siempre presupuestada |
| verticales aplicadas | `Experimental` | modelado validado; baselines de dominio pendientes |

## Claims permitidos y prohibidos

Permitidos:

- firmas algebraicas componibles y no criptográficas;
- igualdad como `Indistinguishable` bajo identidad y perfil concretos;
- actualización incremental diferencialmente exacta dentro del contrato;
- filtrado rápido de candidatos y escalado exacto opt-in;
- reconciliación acotada demostrada en el universo publicado.

Prohibidos:

- “sin colisiones”, “prueba de igualdad” o “prueba de pertenencia”;
- “SHA + firma decide isomorfismo”;
- “otro campo resuelve grafos regulares”;
- garantías probabilísticas para puntos de evaluación fijos;
- ratios de velocidad portables derivados de una sola CPU;
- equivalencia con fingerprints químicos o de similitud sin benchmark de tarea.

## Estado de cierre

La implementación F6.V1–V6 queda completada. La campaña semántica es
reproducible y bloqueante en CI; rendimiento se captura en dos arquitecturas y
los corpus externos se programan semanalmente.

Fase 7 y la publicación siguen bloqueadas por evidencia, no por falta de
harness: faltan la matriz multi-microarquitectura completa, baselines externos
de reconciliación y dominio, intervalos estadísticos consolidados y campañas de
grafos de orden 9 o mayores. Esas tareas no deben modificar claims históricos
ni seleccionar parámetros después de observar resultados sin crear un nuevo
manifiesto.
