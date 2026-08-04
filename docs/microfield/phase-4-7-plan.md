# Plan de F4.7-PACKED-SIMD — lanes persistentes y SIMD primo genérico

Fecha de planificación: 2 de agosto de 2026.

Estado: completada localmente. El resultado y las diferencias materializadas
se documentan en [`phase-4-7-final-report.md`](phase-4-7-final-report.md).

Este archivo conserva el plan aprobado y, por ello, mantiene redacción en
futuro en sus hitos. La implementación final sustituyó el
`PackedStorageProfile` propuesto por `PackedKernelSet::storage_kind()`, evitando
duplicar metadata. El candidato `u32` quedó ejecutable y explícito tras superar
corrección; la evidencia de rendimiento publicada corresponde al perfil `u16`.

Esta extensión posterior al cierre de Fase 4 no sustituye el hito histórico
`F4.7 — Calidad`, ya completado en el plan original. Para evitar ambigüedad, su
identificador autoritativo es `F4.7-PACKED-SIMD`.

## 1. Resultado perseguido

Microfield permitirá convertir una vez un lote de campos primos externos a una
representación SIMD persistente y ejecutar sobre ella una secuencia de sumas,
productos y cuadrados sin volver a extraer o reconstruir cada `F` en cada
operación.

```text
&[F] externo
    │ pack y validación una vez
    ▼
PackedBatch<F>
    └── storage privado alineado u8/u16/u32
           │
           ├── add ───────┐
           ├── mul        │ cero repacking y cero alloc
           ├── square     │ entre operaciones
           ├── mul_assign │
           └── square_assign
                  │
                  ▼
            unpack una vez
                  │
                  ▼
              &mut [F]
```

Aquí **zero-copy persistente** significa que no existen copias, conversiones ni
packing entre operaciones de un pipeline ya preparado. La importación inicial
desde `&[F]` y la exportación final a `&mut [F]` siguen siendo copias explícitas.
Solo un tipo mantenido cuyo layout sea propiedad de Microfield puede usar AoS
zero-copy verdadero, como ya sucede con el kernel especializado de `Fp251V1`.

## 2. Motivación y línea base

F4.6 incorporó bridges AVX2 correctos para primos externos canónicos `u8` y
`u16`. Sus kernels reciben `&[F]`; por seguridad, cada tesela:

1. llama a la extracción segura de cada elemento;
2. llena arrays locales;
3. ejecuta SIMD;
4. almacena un array temporal;
5. reconstruye cada `F`.

Ese diseño es una buena frontera funcional, pero no una representación de
throughput. Al aplicarlo a `Fp251V1`, la conversión por operación produjo una
regresión cercana a 8x frente a su ruta especializada zero-copy. Añadir `u32`
antes de eliminar ese coste repetiría el mismo problema.

F4.7 conservará los bridges directos como fallback correcto para la API normal
de slices, pero añadirá una ruta distinta para lotes que se reutilizan.

## 3. Alcance

### Incluido

- representación packed persistente canónica para lanes `u8` y `u16`;
- ABI packed neutral y seguro separado del ABI ordinario `KernelSet<F>`;
- storage owned bajo `alloc` y vistas sobre storage del consumidor sin `alloc`;
- kernels AVX2 que reciben directamente slices de lanes primitivas;
- paridad funcional packed: `add_into`, `mul_into`, `square_into`,
  `mul_assign` y `square_assign`;
- candidato SIMD genérico `u32` con producto ancho y Barrett vectorial;
- calibración separada de pack, unpack, kernel reutilizado y pipeline;
- preservación de los kernels especializados Fp251 y Goldilocks;
- pruebas de API, layout, aritmética, seguridad, ensamblado y rendimiento.

### Fuera de alcance

- un backend primo AArch64;
- AVX-512, IFMA o radix 52;
- SIMD genérico `u64`: AVX2 no ofrece producto vectorial directo `u64 × u64`;
- exponer slices, punteros o representaciones de lanes al usuario;
- un trait público `unsafe` que prometa layout equivalente a un entero;
- selección automática de perfiles externos sin calibración propia;
- migrar inversión batch, Horner y scans completos a una API packed nueva;
- ampliar todavía el manifiesto TOML a campos primos externos, que sigue siendo
  responsabilidad de Fase 5.

F4.7 dejará preparado el sustrato para que algoritmos derivados puedan operar
sobre batches persistentes posteriormente, pero no duplicará toda la API de
Fase 3 en este corte.

## 4. Principios de diseño

1. **Seguridad de layout:** ningún tipo externo será reinterpretado como un
   entero por `transmute`, cast de slice o supuesto sobre `repr(transparent)`.
2. **Conversión en la frontera:** el codec seguro transforma `F ↔ Lane` solo
   durante pack/unpack.
3. **Composición estática:** codecs, kernels y perfiles son tablas const de
   funciones monomorfizadas; no habrá `dyn Trait`.
4. **Una decisión fuera del loop:** la variante de storage se comprueba una vez
   por operación, seguida de una única llamada indirecta al kernel packed.
5. **Ningún coste en el elemento:** `F` no adquiere flags, backend, punteros ni
   alineamiento artificial.
6. **Compatibilidad antes que promoción:** disponer de un layout ejecutable no
   hace que entre en `Auto`.
7. **API aditiva:** `PackedBatch<F>` conserva su fachada; los detalles físicos
   siguen privados y `PackedLayout` permanece `#[non_exhaustive]`.

## 5. Arquitectura objetivo

### 5.1 Dependencias

```text
__private::VerifiedPrimeCanonical*Field
                 │ constantes + conversiones seguras
                 ▼
backend::prime_profile ─────► kernel::packed (ABI neutral)
                 │                    ▲
                 ▼                    │
backend::x86_prime ───────────────────┘
                                      │
Engine<F> ──► PackingPlan ──► PackedBatch<F>
                                  │
                                  └─ storage tagged privado
                                     AoS | LaneU8 | LaneU16 | LaneU32
```

`kernel::packed` no importará `engine`. `engine::packed` consumirá su metadata
y sus tablas de funciones. Los backends no construirán almacenamiento ni
planes; solo implementarán operaciones sobre slices primitivas ya validadas.

### 5.2 ABI ordinario y ABI persistente

El ABI actual permanece intacto:

```rust
type BinaryKernel<F> = fn(&mut [F], &[F], &[F]);
```

Se añadirá un ABI interno opcional para batches persistentes. Su forma exacta
podrá variar durante la implementación, pero conservará estas propiedades:

```rust
type LaneBinary<L> = fn(&mut [L], &[L], &[L]);
type LaneUnary<L> = fn(&mut [L], &[L]);
type LaneBinaryAssign<L> = fn(&mut [L], &[L]);
type LaneUnaryAssign<L> = fn(&mut [L]);

type Pack<F, L> = fn(&mut [L], &[F]);
type Unpack<F, L> = fn(&mut [F], &[L]);
```

`PackedKernelSet<F>` será un enum interno de factories estáticos —AoS, `u8`,
`u16` o `u32`— que contiene codec y funciones de operación compatibles. No se
usará una factory dinámica ni una tabla registrable por el consumidor.

`KernelSet<F>` podrá asociar opcionalmente una estrategia packed. La ausencia
de esa estrategia conserva exactamente el comportamiento AoS actual.

### 5.3 Storage privado

`PackedBatch<F>` dejará de asumir que la representación física siempre es
`AlignedBuffer<F>`. Internamente usará una suma cerrada:

```text
PackedStorage<F>
├── Aos(AlignedBuffer<F>)
├── CanonicalU8(AlignedBuffer<u8>)
├── CanonicalU16(AlignedBuffer<u16>)
└── CanonicalU32(AlignedBuffer<u32>)
```

Las variantes serán privadas. `PhantomData<F>`, `FieldId` y el plan impedirán
que dos campos nominalmente distintos compartan storage aunque usen el mismo
módulo o tipo de lane.

`AlignedBuffer<T>` ya es genérico y seguirá siendo la única frontera owned de
asignación alineada. El adapter de `MaybeUninit<u8>` se ampliará dentro del
mismo archivo auditado para inicializar slices de lanes sin crear una sexta
frontera `unsafe`.

### 5.4 Plan físico

`PackedLayout` obtendrá variantes únicamente cuando exista un kernel capaz de
ejecutarlas:

- `CanonicalU8`;
- `CanonicalU16`;
- `CanonicalU32`, condicionada a completar F4.7.4.

`PackingPlan` distinguirá:

- tamaño lógico de `F`;
- tamaño físico de la lane;
- longitud lógica;
- longitud padded en lanes;
- elementos por tesela;
- alineamiento;
- bytes físicos exactos;
- identidad de campo, backend y perfil packed.

`element_size()` conservará su semántica actual para no romper consumidores.
Se añadirá un accessor explícito para el tamaño físico. `data_bytes()` pasará a
describir, como ya promete, los bytes realmente ocupados por el storage packed.

El plan dejará de inferir layouts mediante comparaciones ad hoc de
`BackendId`. La estrategia seleccionada entregará un `PackedStorageProfile`
neutral; así un mismo backend podrá ofrecer AoS especializado para un campo y
lanes persistentes para otro.

### 5.5 Operaciones y atomicidad

Se añadirá `add_packed_into` y su equivalente para vistas, completando las
cinco operaciones del ABI batch. No se añadirá `add_assign` únicamente a la
ruta packed mientras no exista en el ABI ordinario.

Antes de escribir se comprobarán:

- backend;
- `FieldId`;
- perfil y layout;
- longitudes lógica y padded;
- tipo de lane;
- alineamiento y capacidad;
- compatibilidad de los tres planes.

Un error dejará la salida completa intacta. Después de validar, se realizará un
solo match de variante y una llamada indirecta; el loop no contendrá checks de
layout, detección de CPU ni conversiones de `F`.

## 6. Perfiles aritméticos

### 6.1 Canónico `u8`

- módulos primos impares `3 <= p <= 251`;
- 32 residuos por tesela AVX2;
- widening a `u16`;
- suma/producto y Barrett con recíproca `floor(2^16/p)`;
- resultado canónico almacenado directamente como `u8`.

### 6.2 Canónico `u16`

- módulos primos impares `3 <= p <= 65_521`;
- 16 residuos por tesela;
- widening a `u32`;
- Barrett con recíproca `floor(2^32/p)`;
- resultado canónico almacenado directamente como `u16`.

### 6.3 Canónico `u32`

- módulos primos impares `3 <= p <= 4_294_967_291`;
- ocho residuos lógicos por tesela;
- dos grupos even/odd de `vpmuludq` para formar ocho productos `u64`;
- cociente Barrett mediante high-half vectorial de `u64 × reciprocal`;
- recíproca `floor(2^64/p)`;
- una corrección modular fija demostrada por `x < p² < 2^64`;
- comparación unsigned y compactación branchless a `u32`.

El perfil `u32` se implementará primero como candidato explícito. Si no existe
una región donde el kernel reutilizado supere al portable, permanecerá privado
o experimental y `PackedLayout::CanonicalU32` no se anunciará como optimizado.
No se sustituirá silenciosamente por una reducción escalar lane a lane.

### 6.4 `u64`

No habrá perfil AVX2 genérico. Goldilocks conserva su reducción Solinas
especializada de cuatro lanes. Otros módulos canónicos `u64` requerirán un
perfil matemático específico, AVX-512DQ o una representación IFMA/radix 52
futura, cada uno con ADR y calibración propios.

## 7. Selección y política de rendimiento

El backend se seguirá seleccionando al construir `Engine`. Crear un
`PackedBatch` no cambiará backend ni ejecutará una segunda detección.

- Fp251 y Goldilocks conservarán sus umbrales automáticos actuales porque sus
  rutas directas ya ganan sin exigir packing.
- Los perfiles externos `u8`, `u16` y `u32` seguirán `explicit_only`.
- Una ganancia exclusiva del pipeline reutilizado no podrá promover una ruta
  para llamadas directas ordinarias.
- F4.7 medirá y publicará el número mínimo de operaciones que amortiza
  pack+unpack; no añadirá todavía un `reuse_hint` a `EngineBuilder`.
- Un futuro selector basado en reutilización requerirá un contrato de workload
  explícito y versionado, nunca una suposición oculta.

## 8. Hitos

| Hito | Entregable | Dependencia | Gate de salida |
|---|---|---|---|
| F4.7.0 | baseline, semántica zero-copy y ADR 0025 | F4.6 | benchmarks directos y persistent pipeline congelados |
| F4.7.1 | ABI `kernel::packed` y metadata de storage | F4.7.0 | sin ciclo `kernel → engine`, sin cambios en API escalar |
| F4.7.2 | storage tagged owned/prestado y nuevos planes | F4.7.1 | round-trip, offsets, padding, overflow, Miri y cero alloc reutilizado |
| F4.7.3 | kernels packed `u8`/`u16` | F4.7.2 | diferencial completo, tails, in-place y ASM sin codec dentro del loop |
| F4.7.4 | candidato AVX2 `u32` | F4.7.3 | reducer probado, oracle `u128`, ASan y selección explícita |
| F4.7.5 | API packed completa y pipelines repetidos | F4.7.3 | cinco operaciones, errores transaccionales y ninguna reasignación |
| F4.7.6 | calibración y decisión por perfil | F4.7.4–5 | intervalos versionados, break-even publicado y mutaciones rechazadas |
| F4.7.7 | calidad, compatibilidad y documentación | todos | CI completa, inventario `unsafe`, ADR aceptado e informe final |

No se avanzará a un hito que dependa de otro cuyo gate no esté verde.

## 9. Detalle de implementación por hito

### F4.7.0 — congelación

- capturar rendimiento del bridge directo `u8`/`u16` actual;
- medir Fp251 y Goldilocks especializados como controles de no regresión;
- añadir benchmarks de cadenas de 1, 2, 4, 8, 16 y 64 operaciones;
- congelar semántica de plan, padding y errores actuales;
- aceptar ADR 0025 solo después de revisar el prototipo del ABI neutral.

### F4.7.1 — ABI neutral

- crear `kernel/packed.rs` sin dependencia de `engine`;
- modelar `PackedStorageProfile` y tablas estáticas por lane;
- asociar una estrategia packed opcional a cada `KernelSet<F>`;
- trasladar la decisión de layout desde `BackendId` a metadata;
- probar que catálogos externos no pueden suministrar codecs o kernels raw.

### F4.7.2 — almacenamiento

- generalizar owned sobre `AlignedBuffer<T>` ya auditado;
- introducir storage y vistas tagged privadas;
- construir slices de lane sobre `MaybeUninit<u8>` con capacidad y alineamiento
  validados;
- inicializar todo padding al cero físico del perfil;
- conservar `Send`/`Sync` únicamente cuando `F` y el storage lo permiten;
- mantener longitud cero sin asignación ni puntero material.

### F4.7.3 — migración `u8`/`u16`

- reutilizar las pruebas algebraicas de F4.6;
- mover solo el hot loop packed a slices de enteros;
- conservar la ruta directa layout-independent para `mul_into` ordinario;
- impedir que Fp251 abandone su kernel AoS especializado;
- comprobar que el ensamblado packed no llama a métodos de conversión.

### F4.7.4 — `u32`

- añadir `VerifiedPrimeCanonical32Field` y estrategia opaca;
- verificar módulo, primalidad ya certificada, recíproca, rangos y metadata en
  `const fn`;
- implementar producto y reducción con schedule fijo;
- evitar extracción escalar de lanes en el kernel;
- mantener candidato fuera de `Auto`, incluso si gana localmente.

### F4.7.5 — fachada y pipelines

- añadir suma a owned y vistas;
- garantizar que operaciones sucesivas preservan perfil y padding;
- cubrir ping-pong entre dos buffers de trabajo reutilizados;
- documentar patrón recomendado: pack → N operaciones → unpack;
- no introducir un objeto pipeline dinámico ni closures almacenadas.

### F4.7.6 — calibración

- separar `pack`, `unpack`, kernel, fachada, pipeline reutilizado y end-to-end;
- medir cada operación y tamaños alrededor de la tesela;
- calcular break-even por número de operaciones;
- fijar perfiles, toolchain, microcode y SHA del artefacto;
- extender `audit_calibration.sh` y sus mutaciones adversarias;
- retirar o dejar explícita cualquier ruta que no supere el gate.

### F4.7.7 — cierre

- actualizar contratos únicamente con capacidades realmente ejecutables;
- actualizar inventario SHA-256 de las mismas cinco fronteras `unsafe`;
- emitir informe final F4.7 con limitaciones y cifras reproducibles;
- comprobar enlaces, doctests, feature matrix y compatibilidad del legado.

## 10. Matriz de pruebas

### Aritmética

| Perfil | Casos mínimos |
|---|---|
| `u8` | exhaustivo para `p = 3, 17, 251` |
| `u16` | exhaustivo para `p = 257`; fronteras y muestras para 769, 4093 y 65521 |
| `u32` | 65537 y 4294967291; bits de base, fronteras y al menos 100.000 pares sembrados |
| mantenidos | Fp251 y Goldilocks contra sus rutas especializadas |

Cada perfil cubrirá suma, producto, square, variantes in-place, conmutatividad,
distributividad, cero, uno y resultado canónico.

### Longitudes y pipelines

Para cada tesela `T`: `0`, `1`, `T-1`, `T`, `T+1`, `2T-1`, `2T`, `2T+1`,
`31`, `32`, `33`, `63`, `64`, `65`, `255`, `256`, `257`, `1024`, `4096` y
`16384`. Se ejecutarán cadenas de 1, 2, 4, 8, 16 y 64 operaciones alternando
buffers.

### Storage y API

- pack/unpack round-trip y repack de igual longitud;
- todos los offsets posibles antes del alineamiento;
- storage exactamente suficiente y un byte insuficiente;
- longitud cero y overflows de longitud, padding y bytes;
- padding inicializado y preservado tras cada operación;
- plan, campo, backend y layout incompatibles dejan salida intacta;
- compile-fail para mezcla de campos, doble préstamo mutable y acceso a lanes;
- owned y vistas producen bytes lógicos idénticos;
- ningún layout implementa `Serialize`.

### Diferencial y oráculos

- packed contra portable escalar;
- packed contra bridge directo F4.6;
- `u32` contra aritmética `u128` independiente;
- corpus Sage para al menos un campo por perfil;
- secuencias aleatorias reproducibles, no solo operaciones aisladas.

### Asignaciones y seguridad

- cero asignaciones después de construir owned;
- cero asignaciones en vistas y storage prestado;
- Miri para codecs, tagged storage, ownership y offsets;
- ASan para `u8`/`u16`/`u32`, tails, in-place y buffers externos;
- inventario de `unsafe` permanece en cinco archivos;
- ninguna implementación externa puede alcanzar intrinsics.

### Features y plataformas

- `no_std` sin features;
- `portable`, `prime-fields` y su combinación sin `alloc`;
- `alloc + portable + prime-fields` para owned;
- `std` con detección;
- MSRV 1.89;
- cross-check AArch64 portable: compila y no anuncia AVX2;
- Clippy `-D warnings`, rustdoc y doctests compile-fail.

## 11. Gates de rendimiento

1. El kernel packed reutilizado no hará conversiones `F ↔ Lane`, asignaciones,
   división ni detección de CPU.
2. Habrá como máximo un match de layout y una llamada indirecta por operación,
   ambos fuera del loop.
3. La fachada packed reutilizada tendrá menos de 3 % de sobrecoste frente al
   kernel de lane directo en lotes grandes.
4. Fp251 y Goldilocks no empeorarán más de 3 % en ninguna región publicada.
5. Un perfil se anunciará como optimización solo si el peor extremo compatible
   acredita al menos 20 % frente al baseline pertinente.
6. Se publicará por perfil el break-even de `pack + N operaciones + unpack`.
7. Una ruta que solo gana después de reutilización permanecerá explícita para
   la API directa.
8. El padding consumirá como máximo una tesela menos un elemento; no existirán
   buffers scratch por operación.
9. El ELF deberá contener las instrucciones SIMD esperadas y carecer de
   asignador, dispatch interno y calls a codecs dentro del hot loop.

## 12. Identidad y estabilidad

- `FieldId` no cambia: el campo, módulo y encoding son los mismos.
- El layout packed no es serialización ni encoding canónico externo.
- `ArtifactId` cambiará para un tipo mantenido solo cuando el nuevo perfil
  packed sea parte autenticada de su artefacto.
- La ABI de código generado binario v3 no cambia.
- Añadir los traits primo `u32` es aditivo; Fase 5 decidirá la versión de ABI
  que los emite automáticamente.
- Los nombres físicos de las variantes son no exhaustivos, pero una variante
  no se publicará antes de tener backend, tests y documentación.

## 13. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| confundir zero-copy con ausencia de pack inicial | terminología contractual y benchmarks end-to-end |
| rama de layout dentro del loop | ABI typed por variante y auditoría ELF |
| suponer layout de un tipo externo | codecs seguros; ningún cast de `F` a lane |
| inflar `PackedBatch<F>` con enum | buffers permanecen en heap; medir tamaño de la fachada |
| romper views sin `alloc` | mantener storage del consumidor y matriz Miri de offsets |
| promover una ruta rentable solo reutilizada | perfiles externos explícitos y selector sin suposiciones |
| `u32` aumenta demasiado el código | especialización por operación, presupuesto ASM y retirada si no gana |
| ampliar `unsafe` | reutilizar exclusivamente `storage.rs` y `x86_prime.rs` |
| alterar algoritmos de Fase 3 | dejarlos fuera hasta disponer de API packed estable |

## 14. Archivos previstos

```text
crates/microfield/src/
├── __private.rs                         # perfil u32 y codecs generados
├── kernel/
│   ├── packed.rs                        # nuevo ABI neutral, sin unsafe
│   ├── catalog.rs                       # estrategia packed opcional
│   └── metadata.rs                      # PackedStorageProfile
├── engine/packed/
│   ├── plan.rs                          # tamaños lógico/físico
│   ├── storage.rs                       # misma frontera unsafe auditada
│   ├── owned.rs                         # storage tagged
│   ├── view.rs                          # vistas tagged privadas
│   └── mod.rs                           # fachada de cinco operaciones
└── backend/
    ├── prime_profile.rs                 # factories seguros
    └── x86_prime.rs                     # intrinsics en frontera existente

crates/microfield/tests/
├── persistent_prime_simd.rs
├── external_prime_avx2_bridge.rs
├── packed_batch.rs
├── packed_views.rs
├── batch_allocations.rs
└── x86_prime.rs

crates/microfield/benches/prime_fields.rs
crates/microfield/calibration/phase47-*.csv
docs/microfield/adr/0025-persistent-prime-lane-storage.md
docs/microfield/phase-4-7-final-report.md
```

No se creará un nuevo archivo de intrinsics que amplíe por accidente el
inventario `unsafe`.

## 15. Definición de terminado

F4.7-PACKED-SIMD estará cerrada cuando:

- `u8` y `u16` ejecuten pipelines persistentes directamente sobre lanes;
- `u32` esté aceptado con evidencia o rechazado explícitamente sin claims;
- las cinco operaciones packed sean funcionalmente completas;
- direct, packed, owned y borrowed coincidan en toda la matriz;
- no haya asignaciones ni conversiones dentro de operaciones reutilizadas;
- la selección automática existente no sufra regresiones;
- los gates de seguridad, MSRV, `no_std`, Miri, ASan, ASM y legado estén verdes;
- calibración y break-even sean reproducibles;
- ADR 0025 pase de propuesta a aceptado y exista informe final.

Después de este cierre, Fase 5 podrá generar perfiles primos externos que
adquieran automáticamente el codec y el candidato packed apropiados sin abrir
la construcción de catálogos ni estabilizar representaciones internas.
