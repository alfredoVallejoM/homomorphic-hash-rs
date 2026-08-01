# Catálogo de patrones

| Patrón | Ubicación | Finalidad | Coste runtime |
|---|---|---|---|
| Newtype | elementos e IDs | Impedir mezcla accidental | Cero |
| Typestate | `spec::model` | Evitar saltar validación | Cero fuera del generador |
| Builder | `EngineBuilder<F>` | Selección explícita e inmutable | Una vez |
| Strategy | `KernelSet<F>` | Intercambiar portable/ISA | Una llamada por lote |
| Static Factory | `KernelCatalog<F>` sellado | Registrar kernels válidos | Cero mutable |
| Facade | raíz y `Engine` | API pequeña y estable | Cero o wrapper medido |
| Adapter | `spec::adapters` | Aislar TOML, FS y Sage | Fuera del hot path |
| Command | `microfield-gen` | Mapear CLI a casos de uso | Fuera del runtime |
| Template Method estático | `BinaryFieldImpl` + `Polynomial128/256<TAIL>` | Reutilizar producto, reducción, cuadrado, inversión y extensión | Monomorfizado |
| Macro de delegación interna | `generated::binary_field` | Emitir impls nominales sin duplicar matemáticas | Cero |
| Unit of Work | emisión | Publicación transaccional | Solo generación |

## Patrones rechazados

- Singleton y service locator: ocultan dependencias y dificultan tests.
- Observer: no existe flujo de eventos que justifique suscripción.
- Visitor: los descriptores versionados no necesitan doble dispatch.
- Registro dinámico de plugins: añade sincronización y ABI inestable.
- `dyn Field`: impide aprovechar tamaños y constantes en compilación.
- Backend almacenado en el elemento: aumenta tamaño y penaliza cada operación.
