# Roadmap corregido de Fases 3–7

Este documento adapta la especificación funcional externa al estado real del
workspace. Conserva un único crate `microfield` dentro del workspace, features
actuales, catálogo sellado y generación ABI v3.

## Fase 3 — cerrada

Algoritmos derivados, workspace tipado, IR de inversión verificado y
benchmarks. Véase [phase-3-plan.md](phase-3-plan.md).

## Fase 4 — campos primos

1. congelar campos de aceptación y certificados de primalidad;
2. separar entero canónico, representación interna y rango lazy;
3. `Fp251V1`, Goldilocks y un primo multi-limb sin forma especial;
4. portable antes de BMI2/ADX, PMULL/NEON o IFMA;
5. reutilizar los planes de Fase 3 sin introducir supuestos de característica
   dos.

## Fase 5 — generación y contextos externos

El puente estático de campos externos ya existe. La fase se concentra en
bundle/lock, caché concurrente segura, CLI independiente y, después, contextos
dinámicos con checks amortizados. Los registros de `Engine` externos siguen
siendo generados y sellados; el consumidor no escribe catálogos raw.

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

### F6.0 — inventario y congelación del legado

- clasificar `algebra`, `topology`, `engine`, `proofs`, `canonizer`, `harness`,
  ejemplos y benchmarks;
- congelar vectores, formatos, complejidad observada y fallos conocidos;
- distinguir comportamiento compatible de comportamiento matemáticamente
  incorrecto;
- retirar claims criptográficos o probatorios que el código no demuestre.

### F6.1 — corrección y extensión sobre campos

- sustituir dependencia directa de `GaloisSignature256` por contratos y
  configuraciones nominales sobre `Field`/`StaticField`;
- usar `Engine`, Horner, scans, inversión batch y tablas de potencias;
- completar casos vacíos, overflow, multiplicidad, factores cero y errores;
- mantener adapters `legacy` solo cuando los bytes y la ley se demuestren;
- deprecar, no ocultar, incompatibilidades corregidas.

### F6.2 — identidades y encoders

Introducir `EncoderId` y `SignatureId`. Campo, encoder, ley, parámetros,
evaluaciones y schema forman la compatibilidad; dos estados incompatibles no se
combinan aunque compartan `FieldId`.

### F6.3–F6.6 — leyes estructurales

- secuencias con longitud y potencia de concatenación;
- multiconjuntos con multiplicidad y conteo de factores cero;
- paridad restringida por capacidad, no por comentarios;
- `Residual` sin presentarlo como prueba criptográfica;
- serialización canónica y migración explícita del legado.

### F6.G0 — especificación de canonización

Antes de código se fijarán modelo de grafo, lazos, multiaristas, etiquetas,
dirección, encoding y forma canónica. El resultado autoritativo será una
permutación/codificación determinista, no un hash probabilístico.

### F6.G1 — algoritmo de referencia exacto

Primera implementación: refinamiento de particiones seguido de
individualización–refinamiento con desempate lexicográfico. Debe terminar en
una etiqueta canónica exacta y producir el mismo resultado bajo cualquier
re-etiquetado de vértices.

### F6.G2 — proceso verificable

- certificado/replay de decisiones de refinamiento;
- comparación exhaustiva en grafos pequeños;
- pares isomorfos/no isomorfos, automorfismos altos y casos adversariales;
- oráculo externo independiente;
- límites de recursos y cancelación explícita.

### F6.G3 — enriquecimiento algebraico seguro

Las firmas de campo pueden acelerar color refinement, ordenar ramas o descartar
candidatos. Una colisión jamás decide equivalencia: cualquier empate vuelve a
comparación estructural exacta. `FieldId`, `EncoderId` y `SignatureId` quedan
registrados en diagnósticos, pero no forman por sí solos un certificado de
canonización.

### Gate de Fase 6

La fase termina cuando el legado mantenido compila sobre la arquitectura nueva,
sus leyes corregidas tienen migración documentada, y la canonización satisface:

```text
canonical(G) == canonical(relabel(G, permutation))
canonical(G) == canonical(H)  <=>  G y H son isomorfos
```

para la matriz aceptada, sin depender de ausencia de colisiones de campo.

## Fase 7 — extensiones y aplicaciones

Torres/extensiones, FFT, reconciliación y backends adicionales se mantienen
como tracks independientes. `BaseEmbedding` será una capacidad separada; no se
amplía retrospectivamente `ExtensionField`. Las transformaciones entre campos
isomorfos serán adapters generados y certificados, no una matriz genérica con
tipos dimensionalmente incorrectos.
