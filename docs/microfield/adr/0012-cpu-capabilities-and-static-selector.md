# ADR 0012 — Capabilities de CPU y selector estático

**Estado:** aceptado.

## Contexto

H4 dejó `Engine<F>` y un ABI seguro de kernels batch, pero solo existía la
estrategia portable. Los identificadores PCLMUL, VPCLMUL y PMULL no podían
confundirse con implementaciones disponibles. Antes de introducir wrappers ISA
es necesario separar cuatro hechos independientes:

1. el backend forma parte del binario;
2. el campo posee una estrategia certificada para ese backend;
3. la CPU actual puede ejecutar todas sus instrucciones;
4. la estrategia satisface la política solicitada.

Una detección dentro de cada operación introduciría ramas, repetiría trabajo y
haría más difícil demostrar que un wrapper ISA nunca se ejecuta sin soporte.
Permitir que un consumidor construya libremente flags de CPU trasladaría esa
misma precondición de seguridad a código no confiable.

## Decisión

`CpuCapabilities` es un value object inmutable con campos privados. Solo puede
obtenerse mediante:

- `CpuCapabilities::detect()` con `std`, usando los macros de detección de Rust;
- `CpuCapabilities::portable_only()` en cualquier configuración, incluido
  `no_std`.

`Architecture` registra `X86_64`, `Aarch64` u `Other`. Los accessors públicos
permiten diagnóstico, pero no existe un constructor público de bits ISA. Los
tests unitarios sí fabrican combinaciones sintéticas dentro del crate para
cubrir exhaustivamente la tabla sin abrir una vía insegura a consumidores.

`EngineBuilder` comienza con capabilities portables. `build()` nunca detecta
implícitamente; `detect()` toma una instantánea real una vez y termina la
construcción. También se puede inyectar una instantánea confiable ya obtenida.
El `Engine` resultante conserva únicamente la referencia al `KernelSet`, la
política y el tamaño esperado: no almacena capabilities ni vuelve a consultar
la CPU.

`KernelCatalog<F>` contiene cuatro slots estáticos: portable obligatorio y
PCLMUL, VPCLMUL y PMULL opcionales. El código generado con ABI 1 o 2 hereda un
catálogo portable mediante un método con implementación por defecto. Los
presets mantenidos pueden sobrescribirlo con su catálogo interno; ningún
consumidor puede construir tablas o registrar punteros.

Un backend forzado se valida en este orden:

1. `BackendNotCompiled`;
2. `BackendUnsupportedByField`;
3. `BackendUnsupportedByCpu`;
4. `PolicyUnsatisfied`.

La selección automática ignora candidatos que fallen cualquiera de esas
dimensiones y usa estas prioridades:

| Política | Comportamiento |
|---|---|
| `Auto` | prioriza caudal y usa `expected_batch` como umbral |
| `LowLatency` | PCLMUL/PMULL antes de estrategias vectoriales |
| `Throughput` | VPCLMUL antes de PCLMUL/PMULL |
| `PortableOnly` | admite únicamente portable |
| `FixedSchedule` | admite únicamente metadata `Fixed` |

`minimum_batch` es una pista de selección, no una precondición. Todo kernel
registrado debe aceptar correctamente cualquier longitud del contrato batch.

## Seguridad y coste

- La detección no aparece en operaciones escalares ni batch.
- Microfield no mantiene estado global mutable, locks, heap, trait objects ni
  registro runtime; la detección estándar puede usar su cache interno.
- La tabla exhaustiva cubre compilación, campo, arquitectura, 32 combinaciones
  de features, cinco políticas y cuatro backends.
- En H2.3 los bits de compilación ISA permanecen desactivados: detectar una CPU
  capaz no publica una implementación inexistente.
- H2.3 conserva `forbid(unsafe_code)`; los futuros wrappers ISA deberán quedar
  detrás de esta frontera ya validada.
- Cada operación batch mantiene una sola llamada indirecta.

## Consecuencias

La selección completa puede probarse antes de introducir `unsafe`. `no_std`
obtiene un comportamiento determinista portable, y `std` habilita detección
explícita. Los errores son suficientemente precisos para distinguir una build
sin backend, un campo no elegible y una CPU incompatible.

H2.4 y H2.5 deberán registrar un backend en dos sitios coherentes: el conjunto
compilado y el catálogo de cada campo certificado. Activar solo uno no permite
seleccionarlo.

## Alternativas rechazadas

- Detección lazy dentro del kernel: repite trabajo y mezcla seguridad con el
  hot path.
- Flags públicos construibles: permiten afirmar soporte inexistente antes de
  entrar en un wrapper ISA.
- Registro global dinámico: añade mutabilidad, sincronización y orden de
  inicialización.
- `Box<dyn Kernel>`: asigna y añade dispatch sin aportar extensibilidad segura.
- Selección por cada tamaño real: cambiaría estrategia durante la vida del
  motor; el diseño exige elegir una vez.
