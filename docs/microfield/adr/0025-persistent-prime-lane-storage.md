# ADR 0025 — Storage persistente de lanes para primos externos

## Estado

Aceptado el 2 de agosto de 2026.

## Contexto

Los bridges AVX2 genéricos de F4.6 son seguros porque convierten cada elemento
externo por valor. Esa misma propiedad introduce dos conversiones por elemento
y operación. El coste puede superar ampliamente al cálculo SIMD, como demostró
el experimento de sustituir el kernel zero-copy de `Fp251V1`.

`PackedBatch<F>` ya ofrece persistencia y alineamiento, pero actualmente
almacena siempre `AlignedBuffer<F>`. Por tanto no elimina las conversiones del
bridge: el kernel continúa recibiendo `&[F]`.

## Decisión

Separar el layout lógico `F` de su storage batch físico mediante:

1. un codec seguro y estático `F ↔ Lane` usado solo en pack/unpack;
2. storage privado tagged para `F`, `u8`, `u16` y `u32` explícito;
3. un ABI packed neutral sobre slices de lanes primitivas;
4. una estrategia packed opcional asociada al `KernelSet<F>` seleccionado;
5. un único match de layout y una llamada indirecta antes del loop.

No se reinterpretará memoria de tipos externos, aunque estos declaren
`#[repr(transparent)]`. Microfield tampoco publicará un trait `unsafe` similar
a `Pod`: modificar código generado en el crate consumidor no debe permitir
crear referencias con layout inválido dentro del runtime.

Los bridges directos continuarán disponibles. La ruta persistente será una
optimización explícita para pipelines que amortizan la conversión inicial.

## Consecuencias esperadas

- las operaciones repetidas trabajan sin packing, heap ni codecs intermedios;
- `PackedBatch<F>` conserva el tipo nominal y la fachada pública;
- el plan pasa a describir tamaño lógico y tamaño físico por separado;
- Fp251 y Goldilocks pueden conservar AoS especializado;
- perfiles externos permanecen fuera de selección automática;
- `engine::packed::storage.rs` y `backend/x86_prime.rs` siguen siendo las
  únicas fronteras `unsafe` afectadas;
- algoritmos derivados podrán adoptar este sustrato en un hito posterior.

## Evidencia de aceptación

- no existe ciclo de dependencias `kernel → engine`;
- los errores de plan son transaccionales con storage tagged;
- Miri y ASan pasan sobre storage, owned, borrowed y perfiles de tres anchuras;
- ELF no contiene codecs, asignador, división o dispatch dentro del hot loop;
- `u16` acredita entre 58,4 % y 72,8 % conservador desde 64 elementos;
- Fp251 packed converge dentro del 3 % frente a su kernel directo en lotes
  grandes, sin sustituir su selección ordinaria.

Los detalles reproducibles están en
[`../phase-4-7-final-report.md`](../phase-4-7-final-report.md).
