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

H4 ampliará el harness para separar:

- producto ancho y reducción interna cuando exista una frontera de benchmark
  que no exponga limbs en la API pública;
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
