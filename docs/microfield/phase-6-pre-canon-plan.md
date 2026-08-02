# Plan ejecutado F6.0–F6.8: firmas estructurales generalizadas

Fecha: 2 de agosto de 2026.

## Objetivo

Rehabilitar todo el legado independiente del modelo de grafo y construir una
capa no criptográfica capaz de resumir suma/paridad, orden y multiconjuntos
sobre campos estáticos, externos generados y contextos runtime validados de
`microfield`. La canonización se mantuvo congelada hasta su discusión F6.G0.
Esa discusión cerró posteriormente con una corrección: el producto principal
de grafos será etiquetado estructural rápido y la búsqueda exacta será optativa.
Véase [phase-6-fast-graph.md](phase-6-fast-graph.md).

## Arquitectura

```text
bytes ──► StructuralEncoder<F> ──► elemento F
                      │
              EncoderId + FieldId
                      │
        ┌─────────────┼──────────────┬────────────────────┐
        ▼             ▼              ▼                    ▼
  Additive       Sequence       Bidirectional       Multiset
  suma+count     Horner+len     forward+reverse     1 o K puntos
        │             │              │                    │
        └─────────────┴──────────────┴────────────────────┘
                      ▼
              MFSG schema 1

 estructura exacta ──► TrackedSequence / TrackedMultiset
 ecuación inversa  ──► AlgebraicResidual (nunca Proof)
```

`structural` depende de las capacidades públicas de `microfield`; el núcleo de
campos no depende de firmas, grafos ni aplicaciones. Los tipos son genéricos y
monomorfizados. No se introduce `dyn Trait` en el hot path.

## Hitos y estado

### F6.0 — inventario y congelación: completado

- inventario de todos los módulos, ejemplos, pruebas y benches;
- clasificación de leyes válidas, adapters compatibles y claims inválidos;
- congelación de 447 tests y encodings históricos;
- retirada de auto-benches rotos sin borrar su fuente histórica.

### F6.1 — base algebraica moderna: completado

- `GaloisSignature256` delega en `Gf2_256HhV1`;
- se preservan tamaño, alineamiento, limbs públicos y bytes little-endian;
- `FiniteField` queda como compatibilidad, no como extensión recomendada;
- la capa nueva funciona sobre campos binarios y primos estáticos.

### F6.2 — encoders e identidad: completado

- `StructuralEncoder<F>` segregado;
- encoders canónico, binario polinómico y entero primo;
- framing, dominio y límite de 16 MiB configurable;
- ruta inline hasta 256 bytes sin heap;
- `EncoderId`, `SignatureId`, `SignatureContext` y `SignatureLaw`;
- adapters explícitos `LegacyLinearEncoderV1` y
  `LegacyAffineEncoderV1`.

### F6.3 — ley aditiva: completado

- estado de suma y contador exacto;
- combinación de particiones;
- ingestión escalar y masiva transaccional;
- documentación explícita de la paridad en característica dos.

### F6.4 — ley de secuencia: completado

- Horner con base no degenerada;
- longitud exacta y concatenación por potencia;
- ingestión escalar/masiva transaccional;
- residuo supuesto sin claim de pertenencia;
- `TrackedSequence` para `pop` exacto.

### F6.5 — ley de multiconjunto: completado

- producto conmutativo y cardinalidad;
- producto no nulo más contador de factores cero;
- offset ligado a la identidad;
- combinación de particiones e ingestión masiva transaccional;
- `TrackedMultiset` con bytes y multiplicidad exactos.

### F6.6 — interoperabilidad y evidencia: completado localmente

- formato `MFSG` schema 1;
- parser estricto y validación de invariantes;
- API legacy de “proof” reclasificada como residuo;
- benchmark mantenido para las cinco firmas;
- matriz específica, exhaustiva, genérica, de compatibilidad y asignaciones.

### F6.7 — campos externos y runtime: completado localmente

- API directa de elementos con `CanonicalElementEncoder`, sin serializar y
  volver a decodificar un elemento ya validado;
- crate fixture que genera GF(2⁹) desde TOML mediante la factory pública;
- `dynamic-fields` como feature opt-in con firmas que poseen un `DynField`;
- mismo `SignatureId` y mismo wire para la misma definición estática/dinámica;
- checks de `FieldId` amortizados solo en la ruta dinámica.

La ruta runtime es una frontera de conveniencia y descubrimiento/configuración.
La ruta recomendada para ejecución repetida es generar un tipo estático y
beneficiarse de monomorfización, layout fijo y backends seleccionables.

### F6.8 — enriquecimiento estructural: completado localmente

- `BidirectionalSequenceSignature`: Horner en ambos sentidos y composición de
  particiones sin conservar los elementos;
- ruta `push_slice` de dos pasadas: dos Horner y una recomposición por lote,
  sin el producto general por elemento de la fórmula incremental;
- `MultiEvaluationMultisetSignature<F, E, K>`: `K` productos en offsets
  distintos y un contador de ceros por coordenada;
- equivalentes dinámicos con número de puntos runtime;
- identidad que liga base, número/orden de puntos y encodings canónicos;
- caso adversarial reproducible donde dos multiconjuntos colisionan en una
  evaluación y quedan separados en la segunda.

## Contratos de error y atomicidad

Todas las operaciones que retornan `Result` calculan en variables locales y
publican al final. Se distinguen: entrada demasiado grande, elemento no
canónico, overflow, reserva fallida, identidad incompatible, base degenerada,
estado vacío, cero ausente, ítem rastreado ausente y wire inválido.

La atomicidad se refiere a errores representables. Un abort del proceso o un
fallo irrecuperable del allocator no forma parte del contrato.

## Rendimiento

- despacho estático para encoder y campo;
- operaciones escalares de `microfield` monomorfizadas;
- cero asignaciones para inputs de hasta 256 bytes en las cinco firmas
  estáticas compactas una vez construido su contexto;
- métodos masivos sin buffers intermedios y con publicación única;
- combinación de particiones para paralelismo en la capa llamadora;
- tracking exacto separado, con coste explícito de `Vec`/`BTreeMap`;
- ninguna afirmación SIMD nueva: la capa se beneficia de los campos y engines
  ya optimizados sin duplicar ISA.

## Gate de esta parte de Fase 6

F6.0–F6.8 se cierran cuando todas las matrices locales terminan en verde y la
documentación no atribuye seguridad, pertenencia ni inyectividad a una firma
finita. La Fase 6 global permanece abierta para completar el track de grafos.

## Discusión F6.G0: cerrada posteriormente

Antes del nuevo código se decidieron:

1. grafo dirigido/no dirigido y posibilidad de ambos;
2. lazos, multiaristas y etiquetas de vértice/arista;
3. normalización de etiquetas y bytes;
4. definición de forma canónica y permutation witness;
5. límites, cancelación y casos adversariales;
6. rol de firmas algebraicas: resultado estructural principal, con colisiones
   declaradas y canonización exacta solo fuera del hot path.
