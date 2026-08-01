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

- algoritmo portable directo frente a fachada portable;
- estrategia portable frente a PCLMUL detectado;
- producto, cuadrado y suma en lotes 1, 8, 64 y 4096;
- coste de validación y dispatch;
- construcción con capabilities portables frente a detección `std`.

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

## Medición local H2.3

Medición del 1 de agosto de 2026, Rust 1.97.1/LLVM 22.1.6, perfil `bench`, Linux
6.18.7 x86-64, Intel Core i7-13700HX y microcode `0x12f`. Criterion ejecutó 30
muestras, 1 s de warm-up y 3 s de medición:

| Construcción de `Engine` | Intervalo observado |
|---|---:|
| capabilities portables | 852,29–868,25 ps |
| capabilities detectadas | 1,0558–1,0602 ns |

La ruta detectada observa el cache interno de los macros estándar tras la
primera consulta; no representa la latencia fría de CPUID. Ambas rutas están
fuera del hot path, no asignan y el `Engine` construido conserva exactamente
la misma operación batch que H4.

Comando reproducible:

```text
cargo +stable bench -p microfield --bench portable_batch -- \
  engine/construction --warm-up-time 1 --measurement-time 3 --sample-size 30
```

## Medición local H2.4

Medición de elegibilidad del 1 de agosto de 2026, Rust 1.97.1/LLVM 22.1.6,
release, Linux 6.18.7 x86-64, Intel Core i7-13700HX y microcode `0x12f`:

| Campo/lote/operación | Portable `Engine` | PCLMUL `Engine` |
|---|---:|---:|
| GF(2¹²⁸)/1 producto | 89,798–89,928 ns | 4,7955–4,9803 ns |
| GF(2¹²⁸)/1 cuadrado | 6,5390–6,6661 ns | 4,1649–4,2017 ns |
| HH-256/1 producto | 376,41–380,92 ns | 11,245–11,265 ns |
| HH-256/1 cuadrado | 10,858–10,932 ns | 7,9176–8,1900 ns |
| Alt-256/1 producto | 359,07–361,97 ns | 11,269–11,368 ns |
| Alt-256/1 cuadrado | 10,749–10,776 ns | 7,9207–7,9840 ns |
| HH-256/4096 producto | 1,4687–1,4811 ms | 39,055–39,333 µs |
| HH-256/4096 cuadrado | 36,668–37,869 µs | 26,763–26,824 µs |

El criterio de registro usa extremos conservadores: límite superior PCLMUL
contra límite inferior portable. El cuadrado de un elemento mejora al menos
35,7 % en 128 bits, 24,6 % en HH-256 y 25,7 % en Alt-256. Producto mejora más
de 18x/31x desde un elemento y aproximadamente 37,5x en HH-256/4096. El umbral
automático publicado es por tanto `minimum_batch = 1` para los tres presets.

Comandos reproducibles:

```text
cargo +stable bench -p microfield --bench portable_batch -- --quick
bash crates/microfield/tools/audit_x86_pclmul.sh
```

El segundo comando compila el harness, inspecciona el ELF y exige instrucciones
PCLMUL sin referencias al asignador ni llamadas indirectas dentro de los
kernels. Los resultados dependen de CPU, frecuencia, microcode y compilador;
no constituyen una garantía de latencia en otro sistema.
