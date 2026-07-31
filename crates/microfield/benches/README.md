# Benchmarks

El vertical H2 incorpora un harness Criterion ejecutable:

```text
cargo bench -p microfield --bench portable_scalar
```

La medición actual separa:

- multiplicación escalar;
- cuadrado;
- `mul_by_x`;
- reducción streaming de 64 bytes;
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
