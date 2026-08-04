# ADR 0016 — Batches packed persistentes y storage alineado

- Estado: aceptado e implementado
- Fecha: 1 de agosto de 2026
- Hito: H2.6

## Contexto

Los kernels escalares, PCLMUL y PMULL actuales reciben AoS y no necesitan
packing. H2.7 evaluará layouts por lanes para VPCLMUL, cuyo coste solo puede
amortizarse cuando un lote se reutiliza. Se necesita antes una frontera estable
que posea o tome prestada memoria alineada, ate el layout al campo y al backend,
y permita operar repetidamente sin asignar ni transformar dentro del kernel.

Publicar ya `Soa64` o un layout híbrido produciría estados que ningún backend
puede ejecutar. También obligaría a estabilizar prematuramente detalles que
dependen de las mediciones de H2.7.

## Decisión

H2.6 publica:

- `PackingPlan`, construido exclusivamente por `Engine::packing_plan`;
- `PackedLayout::Aos` como único layout v1 ejecutable;
- `PackedBatch<F>` bajo `alloc`, con una asignación alineada persistente;
- `PackedBatchView` y `PackedBatchViewMut` disponibles sin `alloc`;
- `required_packed_bytes` y `pack_into_storage` para
  `&mut [MaybeUninit<u8>]` aportado por el consumidor;
- producto, cuadrado y sus variantes explícitas in-place para owned y vistas;
- `PackError` con overflow, longitud, capacidad, alineamiento, backend y plan
  incompatibles diferenciados.

El plan fija `BackendId`, `FieldId`, layout, longitud lógica y padded, tile,
número de palabras, tamaño de elemento y alineamiento. Sus campos son privados
y no implementa serialización. El tipo nominal `F` impide mezclar campos en
compilación; `FieldId` conserva además diagnóstico y defensa estructural.

Los kernels reciben la región padded completa. Todo padding se inicializa con
`F::ZERO` al construir o repackear. Las operaciones reutilizadas realizan una
validación de planes y una llamada a la estrategia ya seleccionada; no asignan,
detectan CPU ni cambian de layout.

## Frontera de seguridad

`engine::packed::storage` es el único módulo packed autorizado a usar `unsafe`.
Contiene dos adaptadores:

1. `AlignedBuffer<F>` conserva el `Layout` exacto, inicializa todos los slots y
   libera una sola vez en `Drop`;
2. el adapter de storage prestado calcula un interior alineado, comprueba la
   capacidad antes de escribir, inicializa cada `F` y devuelve un slice ligado
   al préstamo exclusivo original.

`F: Field` implica `Copy + Send + Sync`; no existe drop glue oculto. Los unsafe
impl `Send`/`Sync` del buffer siguen las capacidades de `F`. El test estructural
rechaza cualquier `unsafe` fuera de los dos adapters ISA y este módulo.

Las vistas contienen referencias Rust normales. No exponen punteros, offsets o
bytes internos; el borrow checker impide dos vistas mutables solapadas y existe
un doctest compile-fail específico.

## Atomicidad

Longitudes, backend y compatibilidad completa del plan se validan antes de
invocar un kernel. Un error conserva salida y storage. El cálculo de capacidad
para storage externo incluye `alignment - 1`, por lo que funciona para todo
offset posible de la dirección base. Longitud cero no asigna ni requiere bytes.

## Consecuencias

- Campos mantenidos y tipos externos ABI 3 usan la misma API.
- `no_std + portable` conserva vistas sin heap; owned requiere `alloc`.
- Para AoS actual, packed aporta persistencia y alineamiento, no una afirmación
  de aceleración. Los benchmarks separan pack, unpack, kernel reutilizado y
  pipeline total.
- H2.7 añade `AosLanePairs` únicamente junto al backend VPCLMUL y conserva su
  selección explícita tras medir el coste total.
- Cambiar de backend requiere repacking explícito; no hay conversión silenciosa.

## Evidencia exigida

- tests diferenciales en los tres presets y los cinco campos externos;
- longitudes cero, tails, offsets de alineamiento y errores transaccionales;
- contador de cero asignaciones en vistas y operaciones owned reutilizadas;
- Miri para storage owned/prestado y AddressSanitizer x86-64/AArch64;
- Clippy, rustdoc, MSRV y matriz `no_std`/`alloc`;
- benchmarks del coste aislado y end-to-end.

## Alternativas rechazadas

- Exponer SoA/híbrido sin kernel: modela estados parcialmente válidos.
- `Vec<F>` como contrato packed: no puede prometer alineamientos futuros.
- `Box<dyn PackedLayout>`: añade dispatch y asignación sin resolver seguridad.
- Repacking implícito en cada operación: oculta coste y evita amortización.
- Exponer el slice padded: estabiliza representación y permite romper padding.
