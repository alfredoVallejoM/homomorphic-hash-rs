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
| Static Strategy Selector | `spec::optimizer` | Elegir reducción/square/inversión desde datos certificados | Cero; solo codegen |
| Capability Snapshot | `CpuCapabilities` | Separar detección confiable de ejecución | Una vez al construir |
| Static Runtime Selector | `EngineBuilder` + `KernelCatalog` | Resolver build/campo/CPU/política antes del lote | Una vez al construir |
| Verified Capability Profile | `VerifiedIsaProfile` + ABI 3 | Asociar layout/reducción a adapters ISA sin abrir el catálogo | Cero; solo metadata/codegen |
| Architecture Adapter | `x86_pclmul` / `x86_vpclmul` / `aarch64_pmull` | Confinar intrinsics y precondiciones ISA | Cero adicional dentro del kernel |
| Value Object | `PackingPlan` | Fijar backend/campo/layout/longitud sin setters | Una comparación por operación |
| RAII / Resource Owner | `AlignedBuffer<F>` / `PackedBatch<F>` | Inicialización y liberación alineada verificables | Una asignación en construcción; cero al reutilizar |
| Borrowed View | `PackedBatchView(Mut)` | Usar storage externo sin heap y expresar aliasing | Cero |

## Patrones planificados para Fase 2

| Patrón | Ubicación prevista | Finalidad | Coste runtime |
|---|---|---|---|
| Factory + Builder ✅ | `generator::BinaryFieldFactory` | Crear tipos GF(2^m) externos desde definición validada | Cero; solo build/codegen |
| Typestate ✅ | definición → validado → planificado → generado | Impedir emisión previa a Rabin y verificación de planes | Cero fuera del generador |
| Versioned Codegen ABI ✅ | módulo generado ↔ runtime | Desacoplar consumidores del IR y backends internos | Cero |
| Capability Snapshot + Selector ✅ | `CpuCapabilities` + `EngineBuilder` | Preparar ISA sin detección en el hot path | Una vez |
| Verified ISA Bridge ✅ | perfil generado + strategy opaca | Habilitar campos externos sin punteros ni claims externos | Una llamada por lote |
| Persistent Packed Batch ✅ | `PackingPlan` + owned/vistas | Amortizar layout y conservar compatibilidad | Validación + una llamada; sin asignación al reutilizar |
| Paired-Lane Strategy ✅ | `AosLanePairs` + `x86_vpclmul` | Procesar dos campos por registro sin exponer limbs | Una llamada por lote; packing persistente explícito |

La factory es estática: produce código y un tipo nominal antes de compilar. No
es un registro runtime ni una factoría de objetos `dyn Field`.

## Patrones rechazados

- Singleton y service locator: ocultan dependencias y dificultan tests.
- Observer: no existe flujo de eventos que justifique suscripción.
- Visitor: los descriptores versionados no necesitan doble dispatch.
- Registro dinámico de plugins: añade sincronización y ABI inestable.
- `dyn Field`: impide aprovechar tamaños y constantes en compilación.
- Backend almacenado en el elemento: aumenta tamaño y penaliza cada operación.
