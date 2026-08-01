# Benchmarks

El harness Criterion cubre los tres campos públicos:

```text
cargo bench -p microfield --bench portable_scalar
```

La medición actual separa:

- multiplicación escalar;
- cuadrado;
- `mul_by_x`;
- reducción de una entrada de anchura doble;
- inversión.

H4 incorpora además:

```text
cargo bench -p microfield --bench portable_batch
```

Este harness separa:

- fachada batch frente a kernel directo;
- coste de validación y dispatch.

## Línea base local H2

Medición orientativa del 31 de julio de 2026, Rust 1.93.1, release, Linux
x86-64 e Intel Core i7-13700HX:

| Operación | Mediana aproximada |
|---|---:|
| multiplicación | 461 ns |
| cuadrado | 12,16 ns |
| `mul_by_x` | 1,57 ns |
| reducción de 64 bytes | 503 ns |
| inversión | 123,22 µs |

Estas cifras no son un gate portable ni una afirmación sobre otras CPUs. Se
conservan como línea base para detectar regresiones en el mismo entorno.

## Medición local H3

Medición orientativa del 31 de julio de 2026, Rust 1.97.1, release, Linux
x86-64 e Intel Core i7-13700HX:

| Campo | Multiplicación | Cuadrado | `mul_by_x` | Reducción doble | Inversión |
|---|---:|---:|---:|---:|---:|
| `Gf2_128V1` | 111,17 ns | 6,97 ns | 0,85 ns | 219,30 ns | 16,36 µs |
| `Gf2_256HhV1` | 460,10 ns | 11,92 ns | 1,59 ns | 518,14 ns | 118,38 µs |
| `Gf2_256AltV1` | 460,04 ns | 11,83 ns | 1,57 ns | 519,89 ns | 116,57 µs |

La medición H3 usa un compilador distinto de la línea base H2; solo permite
detectar cambios gruesos, no atribuir diferencias pequeñas a la
generalización.

## Medición local H4

Medición orientativa del 1 de agosto de 2026, Rust 1.97.1, release, Linux
x86-64, Intel Core i7-13700HX y 4096 elementos:

| Campo/operación | Directo | `Engine` | Diferencia |
|---|---:|---:|---:|
| GF(2¹²⁸) producto | 456,23 µs | 448,55 µs | -1,7 % |
| GF(2¹²⁸) suma | 1,589 µs | 1,587 µs | -0,1 % |
| HH-256 producto | 1,8190 ms | 1,8533 ms | +1,9 % |
| HH-256 suma | 4,294 µs | 4,296 µs | +0,04 % |
| Alt-256 producto | 1,9702 ms | 1,8395 ms | -6,6 % |
| Alt-256 suma | 4,302 µs | 4,295 µs | -0,1 % |

El peor sobrecoste positivo observado es 1,9 %, inferior al gate de 3 %. Los
resultados favorables no se interpretan como aceleración de la fachada: son
variación de compilación, frecuencia y ruido de medida.
