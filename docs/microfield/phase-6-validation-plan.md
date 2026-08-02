# F6.V — validación científica y aplicada de firmas y grafos

Fecha de decisión: 3 de agosto de 2026.

Estado: implementado F6.V1–F6.V6 el 3 de agosto de 2026. La primera campaña y
su clasificación están en
[`phase-6-validation-final-report.md`](phase-6-validation-final-report.md).
Los gates de evidencia multi-CPU y baselines de dominio siguen bloqueando
publicación, aunque el harness reproducible ya esté completo.

## Objetivo

Determinar, mediante hipótesis falsables y artefactos reproducibles:

1. qué propiedades conserva exactamente cada firma;
2. dónde y cómo degenera;
3. qué ventaja aporta frente a baselines apropiados;
4. para qué dominios es una primitiva útil, un producto aplicable o una vía que
   debe descartarse;
5. qué nivel de discriminación, latencia, memoria y coste incremental ofrece el
   motor de grafos antes de escalar a canonización exacta.

No se buscará demostrar seguridad criptográfica ni inyectividad universal. Una
colisión observada es evidencia de un límite; no observar colisiones es solo una
cota empírica sobre el corpus ejecutado.

## Regla científica

Todo experimento deberá declarar antes de ejecutarse:

- hipótesis y resultado que la refutaría;
- unidad de entrada y distribución de datos;
- baseline y motivo de su elección;
- métricas primarias y secundarias;
- umbral de aceptación, cuando exista fundamento para fijarlo;
- controles positivos y negativos;
- seeds, versiones, hardware y comandos;
- tratamiento de errores, timeouts y resultados censurados;
- formato estable del artefacto de salida.

Los resultados se conservarán aunque contradigan la hipótesis. No se elegirán
campos, lanes, rondas o datasets después de mirar el resultado sin registrar el
cambio como un experimento nuevo.

Cada capacidad terminará clasificada como una de:

- `ValidatedApplication`: ventaja aplicada reproducida y límites cuantificados;
- `ValidatedPrimitive`: ley útil y coste competitivo, pero falta un protocolo o
  integración completa;
- `Experimental`: evidencia insuficiente o demasiado dependiente del dominio;
- `Rejected`: no mejora el baseline, degenera fuera del rango útil o su claim no
  puede sostenerse.

## F6.V0 — infraestructura y protocolo

Entregables:

- manifiestos versionados de experimentos, corpus y baselines;
- runner que emita JSON y CSV deterministas para resultados semánticos;
- captura separada de resultados no deterministas de rendimiento;
- inventario de CPU, toolchain, features, threads y memoria;
- repetición, warm-up e intervalos de confianza definidos;
- esquema para registrar colisiones con el par mínimo reproducible;
- comandos offline después de obtener el corpus;
- gates CI rápidos y campañas pesadas programadas.

Los benchmarks vacíos legados se implementarán o retirarán del inventario
mantenido. Ningún gráfico generado manualmente será la única evidencia de un
claim.

## F6.V1 — leyes, dominios exhaustivos y metamorfismo de firmas

La suite actual cubre identidad, composición, atomicidad, wire y generalización
de campos. F6.V1 añadirá una matriz sistemática para:

| Firma | Ley principal | Transformaciones normativas |
|---|---|---|
| aditiva | partición por suma/paridad | reordenación, partición, merge y delta |
| secuencia | concatenación Horner | chunking, asociación, prefijo/sufijo y rollback |
| bidireccional | concatenación en dos orientaciones | reversión, orientación y composición |
| multiconjunto | producto con multiplicidad | permutación, partición, inserción y retirada |
| multievaluación | producto en `K` puntos | todas las anteriores y cambio controlado de `K` |
| residual | recomposición de una ecuación | término correcto, término falso y contexto incompatible |

Gates:

- universos pequeños exhaustivos en F251 y al menos un campo binario;
- árboles de partición de todas las formas hasta el límite elegido;
- equivalencia estática/dinámica para la misma presentación;
- equivalencia scalar/batch/ISA disponible;
- tests metamórficos generados y shrinking del primer contraejemplo;
- cero mutación ante error, overflow o contexto incompatible;
- wire golden y rechazo de cada byte semánticamente inválido.

El residual tendrá un control negativo obligatorio que demuestre que no prueba
pertenencia. No podrá promocionarse como aplicación independiente.

## F6.V2 — colisiones y límites de las firmas

Se medirán por separado:

- colisiones inevitables de la ley, como paridad o pérdida de orden;
- colisiones del encoder antes de entrar al campo;
- colisiones por reducción finita;
- colisiones de una evaluación separadas por evaluaciones adicionales;
- colisiones compartidas entre campos o perfiles supuestamente independientes;
- degeneración causada por parámetros fijos, bases o puntos de evaluación.

Campañas:

1. enumeración exhaustiva de alfabetos pequeños y longitudes/cardinalidades
   crecientes;
2. búsqueda diferencial y por cobertura de colisiones mínimas;
3. distribuciones uniforme, Zipf, alta duplicación, valores adversariales y
   entradas reales;
4. curvas para `K=1..4` y bundles multi-campo;
5. comparación de memoria/latencia frente a la reducción de candidatos.

No se aplicará una cota tipo Schwartz–Zippel como garantía a puntos fijos. Solo
se publicará una cota probabilística cuando el protocolo muestree los puntos de
acuerdo con las hipótesis de la cota y registre el modelo de confianza.

## F6.V3 — aplicaciones de las firmas

### A. Agregación distribuida y deltas

Evaluar aditiva y multiconjunto como resúmenes de particiones de inventario,
telemetría y estado distribuido.

Baselines:

- estado exacto mediante mapa/ordenación;
- XOR/suma modular simple;
- hash criptográfico de la representación ordenada;
- recomputación completa frente a actualización algebraica.

Métricas: throughput, memoria, bytes intercambiados, coste de merge/delta,
colisiones y coste de confirmación exacta.

### B. Reconciliación de conjuntos y multiconjuntos

La firma producto es solo el componente de evaluación de un polinomio
característico. Se implementará un protocolo de recuperación acotada o se
clasificará la capacidad únicamente como `ValidatedPrimitive`.

Se medirán diferencias simétricas de tamaño conocido/desconocido, duplicados,
factores cero, pérdidas, comunicación y coste de decodificación. La comparación
incluirá transmisión exacta y un sketch de reconciliación mantenido.

### C. Secuencias y streams

Evaluar secuencia y bidireccional sobre logs, trazas, paths y fragmentos:

- concatenación de chunks sin releer el prefijo;
- detección de reordenación, inserción, eliminación y reversión;
- composición paralela de fragmentos;
- ventana o rollback solo donde la ley publicada lo permita.

Baselines: fingerprint polinómico convencional, hash criptográfico incremental,
recomputación y comparación exacta.

### D. Índices compactos multi-canal

Medir si combinar leyes independientes reduce suficientemente el conjunto de
candidatos para compensar bytes, evaluaciones y CPU adicionales. La igualdad de
canales seguirá significando `Indistinguishable`.

Un caso de uso solo será validado si mejora al menos una dimensión relevante
—tiempo, memoria, comunicación o actualización— sin ocultar el coste de la
confirmación exacta ni degradar los requisitos de corrección.

## F6.V4 — validación científica del motor de grafos

### Oráculos y familias

- todos los grafos simples no isomorfos hasta 8 vértices;
- campaña programada hasta 9 vértices si cabe en el presupuesto reproducible;
- familias CFI y otras construcciones adversariales para refinamiento local;
- catálogos de grafos fuertemente regulares, Cayley, cospectrales y regulares
  desconectados;
- grafos dirigidos, multigrafos, bucles, etiquetas, roles y multiplicidades;
- hipergrafos sin expansión a cliques;
- normalizaciones, permutaciones y ediciones metamórficas.

Todo resultado exacto se contrastará con al menos un canonizador independiente.
Para cada par:

```text
oracle_isomorphic => ninguna ruta puede responder Different
oracle_non_isomorphic && profile_different => descarte correcto
profile_equal => Indistinguishable, nunca isomorfo
exact_completed => misma decisión y forma estable bajo permutación
budget_exhausted => ninguna forma parcial
```

### Escalones que se medirán por separado

- v1 de campo;
- v1 híbrido con SHA-256;
- perfil global v2;
- v2 adaptativo con motivos;
- evidencia multi-campo;
- canonización exacta con presupuestos crecientes.

Métricas:

- falsos negativos de invariancia/isomorfismo: deben ser cero;
- pares no isomorfos indistinguibles por escalón;
- reducción acumulada de candidatos;
- frecuencia de activación y omisión de motivos;
- tasa de escalado exacto y `BudgetExhausted`;
- nodos de búsqueda, frontier, bytes y asignaciones;
- latencia p50/p95/p99 y memoria pico;
- coste por vértice, incidencia, etiqueta y relación.

## F6.V5 — validación aplicada de grafos

Se mantendrán cuatro verticales con semántica distinta:

1. moléculas: átomos, enlaces, clases y comparación con fingerprints químicos;
2. redes dirigidas etiquetadas: SCC, roles, cambios y comunidades como datos,
   sin presentarlas como propiedades recuperadas por la firma;
3. grafos de conocimiento/relacionales: dirección, tipo de relación,
   multiplicidad y etiquetas;
4. hipergrafos biológicos o de restricciones: roles de incidencia y aridad.

Para cada vertical se medirán deduplicación, renumeración, perturbaciones,
candidate recall, candidate reduction, throughput, memoria y coste incremental.
En química se distinguirá discriminación de igualdad estructural de similitud:
la librería no sustituirá fingerprints de dominio sin demostrar la tarea y la
métrica correspondientes.

La ruta incremental se comparará con recomputación completa sobre ráfagas de
ediciones de 1, 10, 100 y porcentajes crecientes del grafo. Se registrará el
punto de cruce donde deja de ser ventajosa.

## F6.V6 — rendimiento, reproducibilidad y decisión

Matriz mínima:

- x86-64 Intel y AMD de al menos dos microarquitecturas;
- ARM64 real de al menos dos familias cuando sea accesible;
- 1, 2 y varios threads;
- grafos desde pequeños hasta al menos un millón de vértices en casos sparse;
- densidades y regularidades distintas;
- campos F251, un primo mayor y GF(2^256) donde el coste lo permita.

Se separarán preparación, ejecución, encoding, SHA, perfil global, motivos,
actualización y búsqueda exacta. No se agregará un ratio favorable de una ruta
con el coste omitido de otra.

El informe final contendrá:

- tabla de clasificación de cada firma y aplicación;
- catálogo de colisiones mínimas y familias degeneradas;
- curvas de calidad/coste por escalón de grafos;
- resultados brutos y resumen reproducible;
- claims permitidos y claims prohibidos;
- decisión de mantener, rediseñar o retirar cada API experimental;
- requisitos concretos previos a publicación.

## Gates de cierre

F6.V termina únicamente cuando:

1. todas las leyes públicas tienen suite exhaustiva o metamórfica proporcional;
2. cada firma posee un informe de colisiones y un caso aplicado con baseline;
3. la reconciliación se implementa de extremo a extremo o se declara solo
   primitiva;
4. grafos pasan corpus exhaustivo/adversarial y oráculo independiente;
5. se publican tasas de indistinguibilidad por escalón, no solo ejemplos;
6. incrementalidad tiene curvas de punto de cruce;
7. rendimiento se reproduce en más de una familia de CPU;
8. CI ejecuta los gates rápidos y una campaña programada conserva los pesados;
9. toda afirmación comercial o científica queda trazada a un artefacto;
10. el informe puede concluir honestamente que una vía no aporta valor.

Hasta entonces quedan bloqueados Fase 7, estabilización pública, elección de
licencia y publicación. Correcciones de defectos y mejoras necesarias para
ejecutar F6.V sí están permitidas; ampliar el núcleo con nuevas familias no.

## Fundamentación

La evaluación polinómica tiene precedentes reales en fingerprints de secuencia
y reconciliación de conjuntos, y el refinamiento WL tiene aplicaciones medidas
en kernels de grafos. Esos precedentes justifican hipótesis, no validan esta
implementación. F6.V debe demostrar el valor de sus contratos, parámetros y
costes concretos sobre los dominios declarados.
