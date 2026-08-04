# Informe final F4.7-PACKED-SIMD

Fecha de cierre local: 2 de agosto de 2026.

Estado: implementación terminada y gates locales superados. La validación
remota se vincula al commit publicado por GitHub Actions.

## Resultado

Microfield dispone de batches persistentes que separan el tipo lógico `F` de
su representación física. Un campo primo externo verificado puede convertirse
una vez a lanes canónicos `u8`, `u16` o `u32`, ejecutar una secuencia de suma,
producto y cuadrado sin reconstruir `F`, y convertirse de vuelta una sola vez.

```text
&[F] ── pack ──► PackedBatch<F, lane privada>
                    │
                    ├─ add
                    ├─ mul
                    ├─ square
                    ├─ mul_assign
                    └─ square_assign
                    │
&mut [F] ◄─ unpack ─┘
```

No se reinterpretan objetos externos ni se expone su storage. Zero-copy
persistente significa cero conversiones y cero asignaciones **entre**
operaciones; pack y unpack continúan siendo fronteras explícitas.

## Arquitectura materializada

### ABI neutral

`kernel::packed` contiene una suma cerrada `PackedKernelSet<F>` y tablas
monomorfizadas `PackedLaneKernels<F, T>`. `kernel` no depende de `engine` y el
ABI batch ordinario `fn(&mut [F], &[F], &[F])` permanece intacto.

Cada `KernelSet<F>` conserva una estrategia física:

- `Aos` para el comportamiento anterior;
- `CanonicalU8`;
- `CanonicalU16`;
- `CanonicalU32`.

La variante se resuelve una vez en la fachada packed. El hot loop recibe
slices primitivas y no contiene detección de CPU, codecs ni dispatch de layout.

### Storage y vistas

`PackedBatch<F>` usa internamente un enum privado sobre `AlignedBuffer<F>`,
`AlignedBuffer<u8>`, `AlignedBuffer<u16>` o `AlignedBuffer<u32>`. Las vistas
prestadas modelan la misma suma cerrada sobre slices con lifetime exclusivo.

El adapter `MaybeUninit<u8>`:

1. valida longitud, overflow, capacidad y alineamiento;
2. inicializa todos los slots antes de crear una referencia tipada;
3. ejecuta el codec seguro;
4. conserva padding físico cero.

La implementación permanece en `engine/packed/storage.rs`, una de las cinco
fronteras `unsafe` ya auditadas.

### Plan físico

`PackingPlan` distingue ahora:

- `element_size()`: tamaño lógico de `F`, conservado por compatibilidad;
- `physical_element_size()`: tamaño de la lane almacenada;
- `data_bytes()`: bytes físicos exactos, incluido padding;
- layout, backend, `FieldId`, alineamiento y múltiplo de tesela.

La estrategia determina el layout; ya no se deduce únicamente de
`BackendId`. Esto permite que el mismo backend AVX2 use Goldilocks AoS,
Fp251/lane `u8` y perfiles externos con otras anchuras.

## API completada

La fachada owned añade:

```rust
Engine::add_packed_into
Engine::mul_packed_into
Engine::square_packed_into
Engine::mul_packed_assign
Engine::square_packed_assign
```

Las vistas ofrecen sus equivalentes `*_packed_view_*`. Todos los planes se
validan antes de escribir; backend, campo, layout o longitud incompatibles
dejan la salida intacta.

`StaticFieldSpec::__from_generated_prime` permite que código certificado en un
crate consumidor describa honestamente característica, identidad y encoding
antes de construir un plan. No habilita catálogos ni punteros de función.

## Perfiles SIMD

### `u8`

- módulos impares `3..=251`;
- 32 lanes AVX2;
- widening a `u16` y Barrett `floor(2^16/p)`;
- Fp251 usa este storage packed sin modificar su kernel directo especializado.

### `u16`

- módulos impares `3..=65521`;
- 16 lanes AVX2;
- widening a `u32` y Barrett `floor(2^32/p)`;
- perfil externo explícito, nunca promovido por compatibilidad estructural.

### `u32`

- módulos impares `3..=4294967291`;
- ocho residuos lógicos por tesela;
- productos even/odd mediante `vpmuludq`;
- high-half `u64 × reciprocal` construido con cuatro productos `u32`;
- una corrección modular branchless porque `x < p² < 2^64`;
- compactación a `u32` mediante `vpblendd`.

El candidato `u32` queda funcional y explícito. Su corrección está aceptada,
pero no se publica un claim de aceleración ni un umbral automático hasta medir
campos concretos en más familias de CPU.

No se añadió un perfil genérico `u64`; Goldilocks conserva su reducción Solinas
especializada.

## Rendimiento observado

Entorno: Intel Core i7-13700HX, x86-64, AVX2, Linux 6.18.7, benchmark release.
Los intervalos completos están en
`calibration/phase47-packed-i7-13700hx-2026-08-02.csv`.

Producto de un perfil externo canónico `u16`, módulo 65521:

| elementos | directo, límite inferior | packed, límite superior | ganancia conservadora | break-even incluyendo pack/unpack |
|---:|---:|---:|---:|---:|
| 16 | 15,414 ns | 14,549 ns | 5,6 % | 9 operaciones |
| 64 | 57,302 ns | 23,792 ns | 58,4 % | 1 operación |
| 256 | 210,45 ns | 63,092 ns | 70,0 % | 1 operación |
| 1024 | 888,81 ns | 229,28 ns | 74,2 % | 1 operación |
| 4096 | 3518,5 ns | 977,61 ns | 72,2 % | 1 operación |
| 16384 | 13,426 µs | 3,6516 µs | 72,8 % | 1 operación |

En Fp251, donde `F` ya es un byte transparente y el backend directo era
especializado, packed no pretende sustituir la llamada directa aislada. A
16384 elementos, producto directo y packed quedaron dentro del 3 % y no se
observó regresión del kernel existente.

La calibración está ligada a un gate adversario. Alterar intervalos, ganancia o
break-even hace fallar `audit_calibration.sh`.

## Verificación

### Aritmética

- Fp17 exhaustivo en el bridge `u8`;
- Fp251: los 63001 pares posibles en suma y producto directos y packed;
- `u16`: módulo 257 exhaustivo, fronteras y muestras para 769, 4093 y 65521;
- `u32`: módulo 65537 con más de 100000 entradas;
- primo máximo `4294967291`: base binaria completa, fronteras y 100000 lanes;
- longitudes alrededor de teselas y hasta 16384;
- pipelines de 64 operaciones alternando producto y cuadrado.

### Memoria y seguridad

- cero asignaciones después de construir el batch owned;
- borrowed storage probado para todos los offsets de alineamiento;
- padding inicializado antes de crear slices tipadas;
- Miri sobre storage de lanes y vistas packed;
- AddressSanitizer sobre `u8`, `u16`, `u32`, Fp251 y vistas;
- inventario `unsafe` conserva exactamente cinco archivos autenticados.

### Build y plataformas

- `no_std`, portable, binarios y primos en combinaciones independientes;
- `alloc` opcional para owned;
- MSRV 1.89;
- cross-check AArch64 portable con y sin `alloc`;
- Clippy de todos los targets con `-D warnings`;
- auditoría ELF confirma `vpmulld`, `vpackusdw`, `vpmuludq`, `vpblendd` y
  `vzeroupper`, sin división, asignador o dispatch interno en hot loops.

## Decisiones conservadoras

- Los perfiles externos siguen siendo `explicit_only`.
- No se añadió `reuse_hint` al selector.
- La disponibilidad de una lane no altera `FieldId` ni el encoding canónico.
- `PackedLayout` continúa no exhaustivo y no es una serialización.
- Los algoritmos derivados de Fase 3 no se duplicaron todavía sobre packed.
- La generación TOML de campos primos continúa perteneciendo a Fase 5.

## Cierre

F4.7 cumple su objetivo: el bridge genérico deja de pagar conversión por
elemento en cada operación y conserva estabilidad nominal, seguridad de layout
y selección conservadora. ADR 0025 queda aceptado.

El siguiente corte es Fase 5: generación certificada de perfiles primos
externos y selección del codec `u8`/`u16`/`u32` apropiado sin exponer las tablas
internas implementadas aquí.
