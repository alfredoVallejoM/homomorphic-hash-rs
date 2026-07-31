# ADR 0004 — Política de abstracciones de coste cero

**Estado:** aceptado.

## Decisión

En operaciones escalares solo se permiten traits monomorfizados, value objects
transparentes y constantes generadas. Batch admite una selección previa y una
llamada indirecta por lote.

No se permiten trait objects, heap, detección de CPU, locks ni backend state en
el elemento.

## Verificación

- inspección de ensamblado;
- contador de asignaciones;
- tamaño y alineamiento;
- benchmark kernel directo frente a fachada;
- Clippy, Miri y tests diferenciales.

Una abstracción que empeore más de 3 % una ruta medida se rediseña o se mueve
fuera del hot path.
