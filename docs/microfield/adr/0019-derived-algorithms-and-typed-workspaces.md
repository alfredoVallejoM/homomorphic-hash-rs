# ADR 0019: algoritmos derivados y workspaces tipados

- Estado: aceptada
- Fecha: 2 de agosto de 2026

## Contexto

Inversión batch, scans y Horner necesitan estado temporal y reutilizar un
backend ya seleccionado. Un scratch raw añadiría alineamiento, casts y otra
frontera `unsafe`; ampliar `KernelSet` para cada algoritmo mezclaría primitivas
ISA con leyes de nivel superior.

Además, el IR anterior describía una cadena lineal distinta de Itoh–Tsujii,
que era la implementación realmente emitida.

## Decisión

Los algoritmos derivados viven en `algorithms` y dependen de traits pequeños y
de `Engine<F>`. Los planes fijan identidad, backend, longitud y memoria, pero
no contienen buffers. La inversión recibe `BatchInvertWorkspace<'_, F>` sobre
un slice tipado y `BitMaskViewMut`; scans y Horner no requieren scratch.

No se introduce una tabla dinámica de algoritmos. Las dependencias secuenciales
usan operaciones escalares monomorfizadas y Horner reutiliza kernels batch ya
seleccionados. Solo se añadirá una estrategia indirecta si una medición muestra
una región donde compensa.

El generador emite IR v4 con la cadena Itoh–Tsujii exacta. La interpreta
simbólicamente y bloquea la emisión si no alcanza `2^m-2` o lee un slot sin
inicializar.

## Consecuencias

- no aumenta el inventario `unsafe`;
- el compilador garantiza alineamiento del scratch;
- las rutas principales funcionan en `no_std` sin `alloc`;
- un plan ejecutado con otro backend falla antes de escribir;
- cambiar IR altera `ArtifactId`, no `FieldId` ni ABI runtime;
- una futura estrategia tiled puede añadirse sin cambiar semántica pública.

Se rechazan scratch raw, `Box<dyn Trait>`, registro runtime, hilos internos y
transposición implícita.
