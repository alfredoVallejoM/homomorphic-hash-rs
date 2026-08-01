# Estado actual y siguiente plan

Fecha de revisión: 1 de agosto de 2026.

## Diagnóstico ejecutivo

Actualización H2.3: la frontera de capabilities y selección está completa. La
factory binaria genera campos externos optimizados, y esos módulos ABI 1/2
heredan un catálogo portable. `CpuCapabilities` ofrece detección real con
`std` y un límite portable en `no_std`; `EngineBuilder` distingue compilación,
campo, CPU y política antes de fijar una estrategia. El siguiente trabajo es
H2.4, backend x86-64 PCLMUL.

La Fase 1 completa, H0–H4, está integrada en `origin/main`. H4 entró por
fast-forward mediante `1f176ab`; el `main` resultante superó sus cinco jobs en
[`30703842091`](https://github.com/alfredoVallejoM/homomorphic-hash-rs/actions/runs/30703842091).
`EngineBuilder<F>` selecciona un catálogo estático sellado; `Engine<F>` valida
una vez y delega mediante una sola llamada por lote. El backend portable ofrece
suma, producto y cuadrado out-of-place, y producto/cuadrado in-place, sin
`unsafe`, heap, packing, detección de CPU ni paralelismo.

| Área | Estado | Evidencia |
|---|---|---|
| Fase 1 | Cerrada en `main` | `1f176ab`; cinco jobs verdes en `30703842091` |
| API algebraica | Correcto | `F2` y tres campos completos, nominales y monomorfizados |
| Batch H4 | Integrado | catálogo, builder, fachada y backend portable en `main` |
| Errores batch | Transaccional | todas las longitudes se validan antes de escribir |
| `no_std` | Correcto | scalar y batch compilan sin `std` ni `alloc` |
| Asignaciones | Correcto local y remoto | contador externo: cero en las cinco operaciones y tres campos |
| Dispatch | Correcto en ensamblado | dos comparaciones y una llamada indirecta por operación batch |
| MSRV | Correcto en H4 | Rust 1.89, incluida la medición de cero asignaciones |
| Miri H2.3 | Correcto local | 41 tests runtime y cinco compile-fail; consumidor externo: nueve |
| Rendimiento H4 | Gate local superado | peor sobrecoste observado: 1,9 % en producto HH/4096 |
| ISA | No implementado | IDs no implican disponibilidad; las solicitudes se rechazan |
| Factory H2.1 | Implementada | tipos externos nominales 2..=4096, Rabin y emisión atómica |
| Optimizador H2.2 | Implementado | tres reducciones, square dedicado e Itoh–Tsujii |
| ABI de codegen | Compatible 1..=2 | fuente nueva usa v2; helpers v1 se conservan |
| Capabilities H2.3 | Implementado | snapshot no falsificable, detección x86-64/AArch64 y `portable_only` |
| Selector H2.3 | Implementado | cinco políticas y errores separados por build/campo/CPU/política |
| Slots ISA | Estructura completa, ejecución desactivada | ningún backend ISA se marca compilado antes de H2.4/H2.5 |

## Decisiones H4/H2.3 materializadas

1. `kernel` posee el ABI neutral y metadatos, no implementaciones matemáticas.
2. `backend::portable` contiene los bucles seguros y sin asignación.
3. Cada preset registra su `KernelCatalog<F>` estático; un campo externo ABI
   1/2 recibe por defecto un catálogo portable.
4. `BuiltinField` es público para bounds genéricos, oculto y sellado para
   impedir catálogos externos inseguros.
5. `EngineBuilder` conserva política, backend forzado, tamaño esperado y una
   instantánea de capabilities; la selección ocurre una vez.
6. `Engine` es inmutable, `Copy + Send + Sync` y solo almacena referencia a la
   estrategia, política y hint de tamaño.
7. `FixedSchedule` falla de forma tipada: el producto portable actual depende
   de los operandos y no recibe una garantía falsa.
8. `build()` nunca detecta implícitamente; `detect()` consulta la CPU una vez
   con `std`; `portable_only` cubre `no_std`.
9. Un backend forzado diferencia `BackendNotCompiled`,
   `BackendUnsupportedByField`, `BackendUnsupportedByCpu` y política.
10. PCLMUL, VPCLMUL y PMULL devuelven `BackendNotCompiled` hasta que exista una
    implementación compilada y auditada.

La frontera se registra en
[`ADR 0009`](adr/0009-portable-batch-engine.md).
La detección y el selector se fijan en
[`ADR 0012`](adr/0012-cpu-capabilities-and-static-selector.md).

## Cobertura

La suite raíz de Microfield contiene ahora 117 tests de runtime y cinco
doctests compile-fail. El consumidor generado añade nueve tests de runtime y
dos compile-fail; la integración legada conserva tres tests. H4 añadió:

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

H2.2 añade comparación diferencial low-tail/sparse/dense, la matriz
64..=4096, ABI 1..=2 y el fixture denso GF(2¹⁰). Miri ejecuta tanto helpers
internos como los tres campos externos generados.

H2.3 añade 13 tests de runtime: tabla forzada exhaustiva de 491.520
combinaciones, matriz automática, requisitos de features por arquitectura,
detección contra los macros de Rust, fallback conservador, diagnóstico exacto,
cero asignaciones, invariantes de metadata del catálogo y construcción
concurrente determinista. El consumidor externo ABI 2 prueba además que el
método nuevo con implementación por defecto no rompe su código generado.

La matriz CI añade `portable` sin presets, `std + portable` y compilación
cruzada AArch64 tanto `no_std` como `std`. Localmente ambas ramas AArch64 se
validaron construyendo el sysroot desde `rust-src`. SageMath 10.7 bajo
`laboratorio_np` regeneró además los tres vectores mantenidos con diff byte a
byte vacío.

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

H2.3 no añade campos a `Engine` ni modifica sus operaciones. Criterion mide la
construcción con capabilities portables en 852,29–868,25 ps y con detección
cacheada en 1,0558–1,0602 ns en la máquina local. El contador de asignaciones
observa cero en ambas rutas. El detalle y el comando reproducible están en el
[README de benchmarks](../../crates/microfield/benches/README.md).

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

### Cierre de Fase 1

La matriz local y la remota están cerradas: stable, Clippy, rustdoc, features,
MSRV 1.89, Miri, artefactos deterministas y las 447 pruebas de la biblioteca
legada han terminado correctamente. H4 está integrado y no queda trabajo
pendiente dentro del alcance de Fase 1.

Salida: Fase 1 portable completa.

El detalle consolidado está en
[`phase-1-final-report.md`](phase-1-final-report.md).

### Fase 2 en curso

H2.1 ha materializado `BinaryFieldFactory`: un consumidor puede declarar
GF(2^m), validarlo y generar en `build.rs` un tipo nominal con scalar y batch
portable, sin editar Microfield. H2.2 añade el optimizador portable estático y
mantiene v1 como oráculo diferencial. H2.3 añade capabilities confiables,
catálogo ampliado y selector inmutable. Después siguen PCLMUL, PMULL,
`PackedBatch`, VPCLMUL y calibración multi-ISA.

La primera medición local de H2.2 observa mejoras entre 1,6x y 48,6x en las
rutas cubiertas, con 2,8x en la inversión GF(2²³³). Son resultados locales, no
garantías. Entorno, intervalos y comando están en
[`portable-optimizer.md`](portable-optimizer.md).

El orden, los gates y los entregables están en
[`phase-2-plan.md`](phase-2-plan.md).
