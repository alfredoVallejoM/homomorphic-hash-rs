# Informe de cierre F6.0–F6.8

Fecha: 2 de agosto de 2026.

## Resultado

La parte de Fase 6 anterior a canonización está implementada. El paquete raíz
ya posee una capa soportada de firmas homomórficas no criptográficas sobre
campos estáticos mantenidos, campos externos generados y contextos runtime
validados de `microfield`; el legado conserva compatibilidad sin duplicar
aritmética ni presentar relaciones algebraicas como pruebas.

En el momento de este corte histórico, la Fase 6 completa no estaba cerrada y
los hitos F6.G0–G3 quedaban abiertos para la siguiente discusión.

> Actualización posterior: F6.G0–G4 se redefinieron y ejecutaron como motor de
> etiquetado rápido, no como búsqueda exacta predeterminada. Véanse
> [phase-6-fast-graph.md](phase-6-fast-graph.md) y el
> [informe F6.G3](phase-6-g3-final-report.md). El resto de este informe conserva
> la fotografía histórica del cierre F6.0–F6.8.

## Entregables

- módulo público `structural` con cinco firmas segregadas;
- `GaloisSignature256` como adapter byte-compatible de
  `Gf2_256HhV1`;
- encoders estrictos/framed para campos binarios y primos;
- dos encoders legacy explícitos de migración;
- identidades estables de encoder y firma;
- estado, contadores e invariantes completos;
- composición de particiones y concatenación de secuencias;
- ingestión masiva transaccional;
- tratamiento reversible de factores cero;
- variantes rastreadas para orden y pertenencia exactos;
- residuales algebraicos sin claim probatorio;
- wire canónico autocontenido schema 1;
- ingestión directa de elementos ya validados;
- GF(2⁹) externo generado en build time como prueba de no hardcoding;
- adapters runtime opt-in wire-compatibles con sus equivalentes estáticos;
- secuencia Horner bidireccional;
- multiconjunto const-genérico en múltiples puntos de evaluación;
- benchmark Criterion mantenido y retirada de auto-benches inválidos;
- auditoría, ADR y plan de transición.

## Garantías alcanzadas

1. Los campos, encoders, leyes y parámetros incompatibles no se combinan.
2. Los errores representables no publican estados parciales.
3. Inputs inline actualizan las cinco firmas estáticas compactas sin
   asignaciones después de construir el contexto.
4. El producto de multiconjunto no pierde para siempre la información restante
   cuando aparece un cero.
5. La secuencia conserva longitud y tiene una ley exacta de concatenación.
6. Las colisiones de campo no falsifican la pertenencia del adapter rastreado.
7. Los bytes del embedding legacy y su evaluación permanecen congelados.
8. La aritmética de 256 bits tiene una sola implementación mantenida.
9. Un campo externo generado consume todas las firmas sin adapters propios.
10. La misma definición GF(2⁹) estática y dinámica produce bytes `MFSG`
    idénticos para las cinco firmas.
11. Dos multiconjuntos que colisionan en un producto concreto quedan separados
    por una segunda evaluación en el test adversarial mantenido.

## Límites explícitos

- no hay seguridad criptográfica, resistencia adversarial ni MAC;
- la igualdad de firmas no implica igualdad de estructuras;
- un residual no prueba pertenencia;
- `Tracked*` consume memoria proporcional a los datos;
- los encoders framed reducen a un campo finito y necesariamente pueden
  colisionar;
- el wire schema 1 no serializa el contenido exacto rastreado;
- canonizador, Bloom, spectral y dominios siguen siendo experimentales hasta
  F6.G0;
- no se ha añadido paralelismo interno ni un backend ISA específico de firmas.
- varias evaluaciones reducen colisiones accidentales, pero no ofrecen una
  cota criptográfica ni convierten igualdad de estado en igualdad estructural.

## Evidencia

Todos los gates locales han terminado en verde:

| Gate | Resultado |
|---|---|
| `cargo test --workspace --all-features --all-targets` | workspace completo; 450 unit tests legacy bajo todas las features, 3 de compatibilidad, suite Microfield/Fase 5, ejemplos y benches verdes |
| `cargo test -p homomorphic-hash-rs --all-features --test structural_signatures` | 29/29 |
| `cargo clippy --workspace --all-features --all-targets -- -D warnings` | sin warnings |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps` | correcto |
| `cargo miri test ... structural_signatures` | baseline completa 17/17; 8/8 rutas nuevas seleccionadas, incluido GF(2⁹), runtime binario/primo, fail-closed y overflow |
| contador `allocation-counter` | cero asignaciones y cero bytes para encoder inline + actualización de las cinco firmas estáticas |
| `git diff --check` | sin whitespace inválido |

La suite específica cubre las seis familias mantenidas (`Gf2_128V1`, los dos
GF(2²⁵⁶), `Fp251V1`, Goldilocks y primo genérico 256), un GF(2⁹) externo, su
equivalente runtime y un contexto primo runtime. También cubre framing,
identidades,
todos los bytes del header, overflow de los tres contadores, ceros, colisiones,
atomicidad masiva, serialización, compatibilidad y asignaciones. Incluye un
árbol determinista de 4096 entradas por firma y los 63.001 pares valor/offset de
Fp251 para demostrar que cada offset produce exactamente un factor cero.

### Línea base release

Criterion, build release con LTO, Intel Core i7-13700HX, inputs de ocho bytes,
30 muestras, warm-up 1 s y medición 2 s:

| Ley | 64 | 1.024 | 16.384 |
|---|---:|---:|---:|
| Aditiva | 77,5 M elem/s | 104,4 M elem/s | 106,2 M elem/s |
| Secuencia | 9,39 M elem/s | 35,6 M elem/s | 42,8 M elem/s |
| Multiconjunto | 12,9 M elem/s | 12,2 M elem/s | 11,7 M elem/s |

Son una línea base local de las tres firmas iniciales, no thresholds
universales. El benchmark mantenido registra además secuencia bidireccional y
multiconjunto de tres evaluaciones para cuantificar explícitamente su coste.
La secuencia y el
multiconjunto pagan multiplicación de campo por elemento; la aditiva solo
reduce y suma. El benchmark queda registrado para detectar regresiones y
evaluar después batching especializado sin adelantar claims ISA.

Medición exploratoria de las extensiones, mismo perfil release, 10 muestras,
warm-up 250 ms y ventana 500 ms:

| Firma enriquecida | 64 | 1.024 | 16.384 |
|---|---:|---:|---:|
| Secuencia bidireccional, `push_slice` | 16,19 M elem/s | 20,76 M elem/s | 21,47 M elem/s |
| Multiconjunto, 3 evaluaciones | 4,49 M elem/s | 4,65 M elem/s | 4,12 M elem/s |

La primera implementación incremental de la secuencia bidireccional calculaba
`element · base^posición` en cada inserción y solo alcanzaba aproximadamente
4,44 M elem/s a 16.384. La ruta prestada final usa dos pasadas Horner y una
potencia/recomposición por lote: llega a 21,47 M elem/s, una mejora aproximada
de 4,8×, sin buffer ni asignaciones. El coste de tres evaluaciones de producto
es aproximadamente 2,8× el producto simple en lotes grandes, coherente con las
tres multiplicaciones de campo exigidas por elemento.

## Decisión de continuación

El siguiente paso no es optimizar el canonizador histórico. Es cerrar F6.G0:
definir qué grafo se acepta y cuál es el objeto canónico exacto. Sobre esa base
se diseñará un algoritmo individualización–refinamiento autoritativo; las
firmas de esta entrega podrán acelerar colores, particiones o descarte, pero
todo empate terminará en comparación estructural exacta.
