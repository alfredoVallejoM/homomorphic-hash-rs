# Roadmap corregido de Fases 3–7

Este documento adapta la especificación funcional externa al estado real del
workspace. Conserva un único crate `microfield` dentro del workspace, features
actuales, catálogo sellado y generación ABI v3.

## Fase 3 — cerrada

Algoritmos derivados, workspace tipado, IR de inversión verificado y
benchmarks. Véase [phase-3-plan.md](phase-3-plan.md).

## Fase 4 — cerrada

Los tres campos primos, planes Native/Barrett/Solinas/Montgomery, certificados,
corpus Sage, batch y adapters AVX2/BMI2 están implementados. AVX2 para 251 y
Goldilocks es automático desde sus respectivas regiones medidas; los bridges
AVX2 `u8`/`u16` para perfiles externos y BMI2 permanecen explícitos. VPCLMUL
está desenrollado pero tampoco se promueve sin una victoria estable sobre
PCLMUL. Véanse [phase-4-plan.md](phase-4-plan.md),
[phase-4-final-report.md](phase-4-final-report.md) y
[phase-4-6-report.md](phase-4-6-report.md).

## F4.7-PACKED-SIMD — completada antes de Fase 5

Convierte una vez campos primos externos a lanes persistentes y ejecuta
pipelines `u8`/`u16`/`u32` sin repacking. `u32` queda correcto y explícito; no
se le atribuye aceleración sin calibración propia. No reabre la aritmética de
Fase 4 ni sustituye su antiguo hito F4.7 de calidad. Véanse
[phase-4-7-plan.md](phase-4-7-plan.md) e
[informe final](phase-4-7-final-report.md).

## Fase 5 — generación y contextos externos — cerrada

La factory prima, `Proven`/`ProbablePrime`, bundle/lock, caché concurrente,
CLI, contextos binarios/primos, batch con checks amortizados y puente de
exportación están implementados. Los perfiles estáticos seleccionan
`u8`/`u16`/`u32` o Montgomery y reutilizan bridges AVX2/BMI2 como candidatos
explícitos. Los registros de `Engine` externos siguen siendo generados y
sellados; el consumidor no escribe catálogos raw. Véanse
[phase-5-plan.md](phase-5-plan.md),
[ADR 0026](adr/0026-external-prime-and-dynamic-boundaries.md) y el
[informe final](phase-5-final-report.md).

## Fase 6 — legado, firmas estructurales y canonización de grafos

La Fase 6 cambia respecto de la especificación externa: comienza rehabilitando
el código legado completo y añade un track explícito de canonización de grafos.
No se implementará por sustitución masiva; cada ley se congela, corrige y migra
sobre campos `microfield`.

Esta decisión sustituye expresamente `ARCH-109` de la especificación funcional
externa. Se conserva su intención de no contaminar el núcleo algebraico:
`Graph`, el refinamiento y la búsqueda canónica vivirán en una capa de dominio
posterior que consume `microfield`; no pasarán a formar parte de `field`,
`kernel` ni de la representación de los elementos.

Estado a 2 de agosto de 2026: F6.0–F6.8 están implementados localmente y
documentados en [phase-6-legacy-audit.md](phase-6-legacy-audit.md),
[phase-6-pre-canon-plan.md](phase-6-pre-canon-plan.md) y
[ADR 0027](adr/0027-structural-signatures-not-proofs.md). F6.G0–G7 están
implementados: motor estructural, puente legado, ruta a gran escala, estado
incremental, diagnóstico adversarial, canonización acotada y discriminador
global v2. Véanse [phase-6-fast-graph.md](phase-6-fast-graph.md) y
[phase-6-g7-final-report.md](phase-6-g7-final-report.md).

### F6.0 — inventario y congelación del legado — completado

- clasificar `algebra`, `topology`, `engine`, `proofs`, `canonizer`, `harness`,
  ejemplos y benchmarks;
- congelar vectores, formatos, complejidad observada y fallos conocidos;
- distinguir comportamiento compatible de comportamiento matemáticamente
  incorrecto;
- retirar claims criptográficos o probatorios que el código no demuestre.

### F6.1 — corrección y extensión sobre campos — completado

- convertir `GaloisSignature256` en adapter de `Gf2_256HhV1` y construir la API
  nueva sobre contratos nominales `Field`/`StaticField`;
- reutilizar `Pow`, `Invert` y Horner de `microfield`, con composición de
  particiones y operaciones masivas sin duplicar aritmética;
- completar casos vacíos, overflow, multiplicidad, factores cero y errores;
- mantener adapters `legacy` solo cuando los bytes y la ley se demuestren;
- documentar y aislar, no ocultar, las incompatibilidades corregidas.

### F6.2 — identidades y encoders — completado

Introducir `EncoderId` y `SignatureId`. Campo, encoder, ley, parámetros,
evaluaciones y schema forman la compatibilidad; dos estados incompatibles no se
combinan aunque compartan `FieldId`.

### F6.3–F6.6 — leyes estructurales — completado localmente

- secuencias con longitud y potencia de concatenación;
- multiconjuntos con multiplicidad y conteo de factores cero;
- paridad con contador exacto y límites de colisión explícitos;
- `Residual` sin presentarlo como prueba criptográfica;
- serialización canónica y migración explícita del legado.

### F6.7 — generalización de campos — completado localmente

- ingestión directa de elementos para evitar round-trips canónicos;
- fixture GF(2⁹) externo generado por `BinaryFieldFactory` durante el build;
- adapters opt-in sobre `DynField` para las cinco leyes, aislados del hot path;
- identidad y wire idénticos entre representación estática y dinámica del
  mismo campo;
- rechazo transaccional de campos mezclados y encoders de familia incorrecta.

### F6.8 — firmas enriquecidas — completado localmente

- secuencia bidireccional con evaluaciones Horner forward/reverse y ley exacta
  de concatenación;
- multiconjunto en `K` puntos distintos, con producto y contador de ceros por
  coordenada;
- variantes estáticas const-genéricas y variantes dinámicas con puntos
  validados;
- caso de prueba que exhibe una colisión de producto simple separada por la
  segunda evaluación;
- benchmark comparativo registrado sin atribuir seguridad criptográfica.

### F6.G0 — contrato estructural rápido — completado localmente

Se separan etiquetado, firma y canonización exacta. El modelo es un multigrafo
dirigido relacional con etiquetas, roles, bucles y multiplicidades exactos.
Hipergrafos usan nodos de incidencia y no expansión a cliques.

### F6.G1 — motor lineal genérico — completado localmente

`FastGraphLabeler<F, E, K>` ejecuta propagación con productos multi-evaluación
y transcript de rondas en `O(K R (V + I))`. No incorpora índices de entrada.
F251, campos mantenidos y un GF(2⁹) externo generado ejecutan el mismo contrato.

### F6.G2 — perfiles y huella híbrida — completado localmente

`Fast` tiene rondas fijas; `Robust` busca estabilización hasta un máximo.
`analyze_hybrid` combina la firma algebraica con SHA-256 de histogramas de
ronda y relaciones refinadas exactas. `try_canonicalize` emite bytes exactos
solo para una partición discreta y devuelve `SymmetryRemaining` en cualquier
otro caso. No hay búsqueda oculta.

### F6.G3 — rendimiento a gran escala — completado localmente

- `PreparedGraph` precalcula etiquetas iniciales, descriptores, constantes
  afines y tokens de ronda;
- `GraphWorkspace` reutiliza todos los buffers y devuelve vistas prestadas sin
  asignaciones en el camino secuencial caliente;
- evaluación completa AoS, SoA+AVX2 y ambos con segmentación paralela;
- `GraphExecution` ejecuta rangos de vértices deterministas y conserva
  exactamente firma, etiquetas y partición;
- AVX2 queda opt-in: acelera el caso de un hilo, pero AoS+Rayon gana en el host
  de 24 hilos y evita el bridge SoA;
- `CellularGaloisCanonizer::try_analyze` atraviesa el modelo de incidencias y
  el mismo `F251GraphLabeler`; se retiraron las aserciones de la recurrencia
  histórica que ya no representaban el contrato.

### F6.G4 — incrementalidad — completado localmente

- estado owned con todas las capas `0..R` y workspace transaccional;
- auditoría semántica fail-closed de vértices y ambas filas CSR;
- invalidación por radio y recomputación solo de valores realmente afectados;
- retirada/inserción algebraica de factores, incluido el caso cero;
- índice lineal de dependencias con máximo `2I` registros;
- composición y descomposición exacta de componentes tras editar aristas;
- partición persistente `O(V + C log C)` y diferencial multi-campo completo.

### F6.G5 — robustez adversarial — completado localmente

- diagnóstico exacto de aliasing de campo frente a ambigüedad local;
- umbral versionado de alta regularidad y recomendación de escalado;
- perfiles multi-campo ligados a `GraphEvidenceProfileId`;
- normalización aleatoria, renumeraciones masivas y oráculo exhaustivo hasta
  cinco vértices;
- 35 pares regulares contrastados con SageMath 10.7, con primera colisión no
  isomorfa en seis vértices, más el par fuertemente regular
  Shrikhande/torres 4×4.

### F6.G6 — canonización exacta optativa — completado localmente

`canonicalize_exact` usa individualización–refinamiento exacto, DFS iterativo y
límites independientes de nodos y estado retenido. Solo publica
`DiscreteCanonicalForm` tras recorrer el árbol completo; en otro caso devuelve
`BudgetExhausted`. Nunca sustituye a la firma rápida ni bloquea el camino
predeterminado. El cierre está en
[phase-6-g5-g6-final-report.md](phase-6-g5-g6-final-report.md).

### F6.G7 — discriminación global rápida — completado localmente

- v1 permanece estable y componible; v2 es la fachada recomendada;
- componentes débiles, SCC, tamaños, labels, relaciones, grados, bucles,
  multiplicidades, soporte y rango cíclico forman un descriptor global exacto;
- alta regularidad activa triángulos y `K4` solo bajo una cota de trabajo
  invariante;
- la búsqueda exacta se descompone por componentes y consume un presupuesto
  global restante;
- Graph Atlas, MUTAG, email-Eu-core y diseasome forman un corpus externo fijado
  por SHA-256 y ejecutado opt-in.

El diseño se fija en [ADR 0030](adr/0030-global-v2-and-external-corpus.md).

### Gate de Fase 6

La fase termina cuando el legado mantenido compila sobre la arquitectura nueva,
sus leyes corregidas tienen migración documentada, y cada contrato satisface:

```text
signature(G) == signature(relabel(G, permutation))
label_G[v] == label_relabel_G[permutation(v)]
si try_canonicalize(G) produce Canonical:
    canonical(G) == canonical(relabel(G, permutation))
si canonicalize_exact(G, budget) produce Exact:
    canonical(G) == canonical(relabel(G, permutation))
```

Las firmas finitas mantienen colisiones declaradas; ninguna igualdad se presenta
como prueba de isomorfismo.

## Fase 7 — extensiones y aplicaciones

Torres/extensiones, FFT, reconciliación y backends adicionales se mantienen
como tracks independientes. `BaseEmbedding` será una capacidad separada; no se
amplía retrospectivamente `ExtensionField`. Las transformaciones entre campos
isomorfos serán adapters generados y certificados, no una matriz genérica con
tipos dimensionalmente incorrectos.
