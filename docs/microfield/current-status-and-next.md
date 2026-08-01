# Estado actual y siguiente plan

Fecha de revisión: 1 de agosto de 2026.

## Diagnóstico ejecutivo

H0–H3 están integrados en `origin/main`. H3 entró por fast-forward mediante
`78d517f`; la rama y el `main` resultante superaron sus cinco jobs en
[`30624475704`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30624475704)
y
[`30701163784`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30701163784).

H4 está implementado y validado localmente en `agent/h4-portable-batch`.
`EngineBuilder<F>` selecciona un catálogo estático sellado; `Engine<F>` valida
una vez y delega mediante una sola llamada por lote. El backend portable ofrece
suma, producto y cuadrado out-of-place, y producto/cuadrado in-place, sin
`unsafe`, heap, packing, detección de CPU ni paralelismo.

| Área | Estado | Evidencia |
|---|---|---|
| H3 | Integrado | `78d517f`; rama y `main` con cinco jobs verdes |
| API algebraica | Correcto | `F2` y tres campos completos, nominales y monomorfizados |
| Batch H4 | Implementación local completa | `KernelSet`, catálogo, builder, fachada y backend portable |
| Errores batch | Transaccional | todas las longitudes se validan antes de escribir |
| `no_std` | Correcto | scalar y batch compilan sin `std` ni `alloc` |
| Asignaciones | Correcto localmente | contador externo: cero en las cinco operaciones y tres campos |
| Dispatch | Correcto en ensamblado | dos comparaciones y una llamada indirecta por operación batch |
| MSRV | Correcto en H4 | Rust 1.89, incluida la medición de cero asignaciones |
| Miri | Correcto en H4 | 26 tests de runtime habilitados y cuatro compile-fail |
| Rendimiento H4 | Gate local superado | peor sobrecoste observado: 1,9 % en producto HH/4096 |
| ISA | No implementado | IDs no implican disponibilidad; las solicitudes se rechazan |

## Decisiones H4 materializadas

1. `kernel` posee el ABI neutral y metadatos, no implementaciones matemáticas.
2. `backend::portable` contiene los bucles seguros y sin asignación.
3. Cada tipo generado registra su `KernelCatalog<F>` estático.
4. `BuiltinField` es público para bounds genéricos, oculto y sellado para
   impedir catálogos externos inseguros.
5. `EngineBuilder` conserva política, backend forzado y tamaño esperado; la
   selección ocurre una vez.
6. `Engine` es inmutable, `Copy + Send + Sync` y solo almacena referencia a la
   estrategia, política y hint de tamaño.
7. `FixedSchedule` falla de forma tipada: el producto portable actual depende
   de los operandos y no recibe una garantía falsa.
8. PCLMUL, VPCLMUL y PMULL devuelven `BackendUnavailable` hasta que exista una
   implementación compilada y auditada.

La frontera se registra en
[`ADR 0009`](adr/0009-portable-batch-engine.md).

## Cobertura

La suite de Microfield contiene ahora 81 tests de runtime, cuatro doctests
compile-fail y tres tests de compatibilidad legada. H4 añade:

- equivalencia batch/escalar para los tres campos;
- tamaños `0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 255, 256, 1024,
  16384`;
- suma, producto y cuadrado out-of-place;
- producto y cuadrado in-place;
- canarios antes y después de todos los buffers de salida;
- error exacto y salida intacta para cada forma de longitud incompatible;
- slices vacíos válidos;
- metadatos, políticas, catálogo compartido y backend forzado;
- compile-fail que impide acceder a `KernelSet` o construir catálogos;
- contador aislado que observa cero asignaciones y cero bytes asignados;
- compilación `builtin-fields` sin activar el motor portable.

La compilación `no_std` sin `alloc` proporciona una barrera estructural y el
feature opcional `count-allocations` añade medición dinámica sin introducir
`unsafe` en Microfield. El contador se activa solo en el gate de pruebas.

## Rendimiento H4

Criterion, Rust 1.97.1, release, Linux x86-64, Intel Core i7-13700HX, lotes de
4096 elementos:

| Campo/operación | Bucle directo | `Engine` | Diferencia observada |
|---|---:|---:|---:|
| GF(2¹²⁸) producto | 456,23 µs | 448,55 µs | -1,7 % |
| GF(2¹²⁸) suma | 1,589 µs | 1,587 µs | -0,1 % |
| HH-256 producto | 1,8190 ms | 1,8533 ms | +1,9 % |
| HH-256 suma | 4,294 µs | 4,296 µs | +0,04 % |
| Alt-256 producto | 1,9702 ms | 1,8395 ms | -6,6 % |
| Alt-256 suma | 4,302 µs | 4,295 µs | -0,1 % |

El peor sobrecoste positivo queda por debajo del gate de 3 %. Las diferencias
favorables, especialmente Alt-256, se consideran ruido/variación de código y
no una mejora atribuible a la fachada.

El desensamblado del producto HH muestra dos comparaciones de longitud antes de
`call *0x8(%rax)`: una única llamada indirecta al kernel por invocación. El
kernel contiene una llamada directa al producto ancho y ninguna indirección o
referencia al asignador.

## Hallazgos abiertos

### Media — Alcance de «transaccional»

La publicación de artefactos no promete durabilidad frente a caída del sistema,
publicación concurrente ni atomicidad entre filesystems. Requerirá una política
de coordinación antes de uso concurrente.

### Fuera de alcance

El paquete legado conserva 447 tests correctos, pero sus benchmarks y ejemplos
históricos impiden todavía `cargo check --workspace --all-targets`; también hay
formato legado pendiente. La matriz aislada de Microfield y compatibilidad no
está afectada.

## Siguiente orden

### Cierre de H4

La matriz local está cerrada: stable, Clippy, rustdoc, features, MSRV 1.89,
Miri, artefactos deterministas y las 447 pruebas de la biblioteca legada han
terminado correctamente. Solo queda:

1. crear y publicar el commit H4;
2. exigir CI verde e integrar en `main` en un paso separado.

Salida: Fase 1 portable completa.

### Fase posterior

Añadir PCLMUL y PMULL como adaptadores internos, con detección una vez,
wrappers seguros, vectores idénticos al portable y benchmarks reproducibles.
