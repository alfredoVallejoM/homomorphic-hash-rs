# ADR 0009: motor batch portable con selección previa

- Estado: aceptado
- Fecha: 1 de agosto de 2026

## Contexto

H4 necesita procesar slices sin repetir validación por elemento, introducir
dispatch escalar ni acoplar el motor a PCLMUL/PMULL. También debe admitir una
ruta in-place explícita y conservar la salida intacta ante errores.

## Decisión

`kernel` define una tabla interna `KernelSet<F>` de funciones seguras sobre
slices y metadatos neutrales. `backend::portable` aporta bucles genéricos
monomorfizados. Cada campo generado registra estáticamente un
`KernelCatalog<F>` mediante el trait público, oculto y sellado `BuiltinField`.

`EngineBuilder` selecciona una tabla una vez. `Engine` conserva una referencia
inmutable y cada operación ejecuta:

1. una validación de longitudes;
2. una llamada por puntero de función;
3. un bucle que no asigna ni detecta CPU.

Los identificadores ISA son solicitudes honestas: existen para configuración
y diagnóstico, pero se rechazan como no disponibles hasta que haya un backend
compilado y auditado. La política `FixedSchedule` también falla explícitamente
porque el portable actual es dependiente de datos.

## Consecuencias

- `Engine` no importa ningún backend concreto.
- Los usuarios no pueden construir ni registrar `KernelSet`.
- El borrow checker separa rutas out-of-place e in-place.
- Los slices vacíos son operaciones válidas.
- El motor es `Copy + Send + Sync` y no contiene heap, locks o estado mutable.
- Añadir ISA modifica adaptadores y catálogos, no la API de los elementos.

## Alternativas rechazadas

- `dyn Kernel`: una vtable y objetos no aportan nada a una tabla fija.
- Selección dentro del bucle: repite ramas y detección por elemento.
- Backend dentro del elemento: altera layout y penaliza operaciones escalares.
- Aceptar silenciosamente `FixedSchedule`: crearía una garantía de timing
  falsa.
