# ADR 0032 — assurance y característica de campo en firmas G11

Fecha: 3 de agosto de 2026.

Estado: aceptado.

## Contexto

Las firmas históricas mezclaban tres conceptos: igualdad de un residuo finito,
exactitud sobre elementos de campo ya codificados y tracking exacto de valores
fuente. Además, se asumía informalmente que un campo binario más ancho siempre
discriminaría mejor. Eso es falso para leyes que suman conteos: en
característica dos toda multiplicidad se reduce a paridad.

G11 añade patterns, moments, trazas, determinantes evaluados y contracciones
theta. Ninguno puede adquirir autoridad sobre la forma canónica exacta.

## Decisión

1. Toda familia publica `SignatureAssurance`.
2. `BoundedExactOverEncodedElements` se limita explícitamente a coeficientes ya
   codificados y a la cardinalidad indicada.
3. Cada lane usa hash-to-field independiente, ligado a perfil y canal.
4. La compresión aditiva de conteos se conserva, pero no se recomienda en
   característica dos.
5. La compresión multiplicativa contabiliza factores cero y es la opción
   binaria mantenida para multiconjuntos de patterns.
6. F251/Goldilocks son los perfiles preferidos para conteos aditivos, traces y
   theta sobre operadores simétricos.
7. Campo, encoder, lanes, catálogo y parámetros forman parte de la identidad.
8. Todos los canales G11 son aceleradores de rechazo/routing. Solo `Microcanon`
   y mappings verificados deciden isomorfismo.

## Evidencia

En las 6.177 clases del holdout n=8:

- patterns aditivos GF(2²⁵⁶): 505 salidas;
- patterns producto GF(2²⁵⁶): 5.878, igual que el catálogo exacto;
- matrix GF(2²⁵⁶): 16;
- theta GF(2²⁵⁶): 1;
- theta Goldilocks: 6.170;
- bundle Goldilocks: 6.177.

El bundle aún colisiona en el par CFI adversarial, de modo que cero colisiones
en n=8 no se interpreta como completitud.

## Consecuencias

- elegir el campo pasa a ser una decisión semántica además de una decisión de
  rendimiento;
- GF(2^m) conserva gran valor para productos, secuencias y SIMD, pero no se usa
  automáticamente en todo channel;
- se mantienen dos leyes de compresión porque sus costes y comportamiento por
  característica son distintos;
- futuros perfiles deben demostrar su curva de colisión en discovery y holdout;
- cambiar un acelerador nunca cambia bytes canónicos ni mappings exactos.
