# Plan ejecutado de Fase 3

Fecha de cierre técnico: 2 de agosto de 2026.

## Objetivo

Construir algoritmos derivados reutilizables sobre `Field` y `Engine<F>` sin
reabrir representación, identidad ni selección ISA. La ruta principal debía
ser `no_std`, segura, sin heap y transaccional ante errores de validación.

## Hitos

| Hito | Entrega | Estado |
|---|---|---|
| H3.0 | IR v4 de inversión Itoh–Tsujii y verificador simbólico | Completo |
| H3.1 | `BitMask`, vistas prestadas y workspace tipado | Completo |
| H3.2 | inversión batch tolerante a cero, owned e in-place | Completo |
| H3.3 | productos prefijo/sufijo inclusivos y exclusivos | Completo |
| H3.4 | Horner en las dos orientaciones y `mul_add_into` | Completo |
| H3.5 | potencias de base fija prestadas y owned | Completo |
| H3.6 | pruebas, asignaciones, benchmarks, CI y documentación | Completo |

## Trazabilidad con la especificación funcional externa

La Fase 3 no vuelve a implementar fronteras que ya quedaron cerradas y medidas
en Fase 2. Los kernels dedicados de `square` portable, PCLMUL, VPCLMUL y PMULL,
las vistas packed, sus tails, la selección por backend y las regiones
AoS/packed pertenecen a ese corte previo. Fase 3 los consume a través de
`Engine<F>` y conserva sus pruebas diferenciales.

La propuesta externa de añadir un segundo `AlgorithmSet<F>` dinámico y scratch
de bytes se adaptó deliberadamente: los algoritmos secuenciales se
monomorfizan, los lotes reutilizan el `KernelSet` ya seleccionado y la única
memoria temporal actual es un slice tipado de `F`. Esto satisface el contrato
de alineamiento y evita una indirección y una frontera `unsafe` sin beneficio
medido. Tiling o estrategias packed específicas de un algoritmo derivado se
añadirán únicamente cuando un benchmark reproducible demuestre una región
favorable; no son comportamiento simulado de la API actual.

## Decisiones de diseño

1. `algorithms` depende de capacidades algebraicas y del facade `Engine`; no
   importa módulos ISA.
2. `BatchPlan<F>` fija `BackendId`, `FieldId`, longitud, revisión de algoritmo
   y `WorkspaceLayout`. Un plan no contiene buffers ni detecta CPU.
3. El scratch de inversión es `&mut [F]`, no bytes ni `MaybeUninit`. El tipo
   garantiza tamaño del elemento y alineamiento sin ampliar `unsafe`.
4. La máscara usa un bit por entrada. `count_ones` ignora padding aunque el
   almacenamiento prestado contenga basura fuera de la longitud lógica.
5. Toda precondición falible se valida antes de escribir. Una longitud,
   backend, máscara o workspace incorrectos dejan salida y máscara intactas.
6. Los coeficientes de Horner se ordenan por grado ascendente. La orientación
   «muchos polinomios» exige `PolynomialMajor` o `CoefficientMajor`; nunca
   transpone de forma implícita.
7. Los helpers con heap llevan semántica explícita (`*_alloc`, tipos owned).
   Los planes y rutas prestadas no asignan.
8. No se añadió `AlgorithmSet` con punteros escalares: la medición no justificó
   una segunda indirección. Las primitivas batch siguen en `KernelSet` y los
   algoritmos derivados se monomorfizan o reutilizan el engine seleccionado.

## Contratos matemáticos

La inversión batch sustituye cada cero por uno en el producto acumulado,
invierte una sola vez y reconstruye hacia atrás. Para cada índice:

```text
mask[i] = 1  => out[i] * input[i] = 1
mask[i] = 0  => input[i] = 0 y out[i] = 0
```

Los scans implementan, en ambos sentidos, las variantes inclusiva y exclusiva.
Horner evalúa `c[0] + c[1]x + ...` y rechaza un shape con cero coeficientes;
un conjunto vacío de puntos o polinomios es válido.

## IR v4 de inversión

El plan anterior describía una cadena binaria lineal, aunque la fuente emitida
invocaba Itoh–Tsujii. IR v4 registra las operaciones reales:

- guardar acumulador;
- cuadrados repetidos;
- multiplicar por el valor guardado;
- multiplicar por la base cuando lo exige la descomposición binaria;
- cuadrado final.

Antes de calcular `ArtifactId`, el planner interpreta exponentes simbólicamente,
rechaza lecturas de slots no inicializados y comprueba exactamente
`2^degree - 2`. También publica multiplicaciones, cuadrados y valores guardados.
Los tres artefactos mantenidos usan schema 2/IR 4; el ABI de fuente permanece
en v3 porque la interfaz runtime no cambió.

## Cobertura

- tres campos mantenidos;
- tamaños `0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256,
  1024, 16384`;
- todos cero, ninguno, alternos, cero por tile y extremos cero;
- rutas out-of-place, in-place, prestadas y owned;
- igualdad portable/PCLMUL/VPCLMUL o PMULL cuando el hardware lo permite;
- errores transaccionales de longitud, máscara, workspace y backend;
- scans, layouts de Horner, puntos cero/uno, constantes y shapes vacíos;
- IR correcto en grados 2..=4096 representativos y planes inválidos rechazados;
- cero asignaciones medido en todas las rutas prestadas;
- `no_std` sin `alloc`, Clippy, rustdoc, Miri y ASan integrados en la matriz.

## Rendimiento observado

Medición Criterion rápida, release, Intel Core i7-13700HX, GF(2²⁵⁶) HH. Es
evidencia local y no modifica el selector:

| Longitud | Inversas separadas | Batch prestado | Relación aproximada |
|---:|---:|---:|---:|
| 2 | 103,8 µs | 104,4 µs | paridad |
| 4 | 293,3 µs | 92,9 µs | 3,2× |
| 64 | 5,72 ms | 170 µs | 33,6× |
| 1024 | 96,8 ms | 1,36 ms | 71,3× |
| 16384 | 1,56 s | 20,8 ms | 75,1× |

Se publica un cruce conservador local de cuatro elementos. No se codifica como
regla universal: otras arquitecturas deben ejecutar `derived_algorithms`.

## Salida

Fase 3 deja una base común para campos primos: ninguno de los algoritmos
depende de característica dos salvo la cadena generada concreta de los campos
binarios. No crea hilos, no realiza detección dentro de operaciones y no añade
nuevas excepciones `unsafe`.
