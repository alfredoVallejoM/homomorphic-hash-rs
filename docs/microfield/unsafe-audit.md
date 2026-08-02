# Auditoría de `unsafe` de Fase 2

## Resultado

Fase 2 cierra con `#![deny(unsafe_code)]` en la raíz y cuatro excepciones
locales. No existe `unsafe` en álgebra portable, encoding, identidad, selector,
factory, codegen ni API pública.

| Frontera | Motivo | Precondición | Evidencia |
|---|---|---|---|
| `backend/x86_pclmul.rs` | intrinsics PCLMUL | snapshot real con `pclmulqdq` | diferencial, ASan y ELF |
| `backend/x86_vpclmul.rs` | intrinsics AVX2/VPCLMUL | `pclmulqdq + avx2 + vpclmulqdq` | diferencial, ASan, tails y ELF |
| `backend/aarch64_pmull.rs` | intrinsic PMULL | snapshot real con NEON + PMULL | diferencial ARM64, ASan y ASM |
| `engine/packed/storage.rs` | asignación alineada y slices tipados | layout probado, ownership único e inicialización total | Miri, ASan, canarios y vistas |

Los wrappers ISA reciben elementos Rust válidos por valor. No cargan punteros
SIMD desde memoria del usuario, no exponen representaciones vectoriales y solo
son alcanzables después de que `EngineBuilder` valide arquitectura y features.
El storage inicializa también el padding antes de construir cualquier slice.

## Gate de revisión

`unsafe/unsafe-inventory-v1.sha256` fija el hash de las cuatro implementaciones
revisadas. `tools/audit_unsafe_scope.sh` falla cuando:

- aparece una quinta frontera;
- cambia cualquiera de los cuatro archivos sin actualizar el inventario;
- cambia el número de excepciones al lint;
- falta documentación `SAFETY` en una frontera.

Actualizar un hash no es un arreglo mecánico: exige revisar precondiciones,
aliasing, inicialización, alineamiento, target features, unwinding y ownership;
después deben repetirse Miri, ASan, diferencial y auditoría de ensamblado según
la frontera afectada.

## Resultado de los revisores dinámicos

- Miri: portable completo, almacenamiento owned y rutas generadas ABI 3.
- ASan x86-64: PCLMUL, VPCLMUL, packed owned/prestado y campos externos.
- ASan AArch64: PMULL y packed sobre hardware nativo.
- Los tests estructurales rechazan expansión del alcance y los scripts ISA
  rechazan asignador, división y dispatch indirecto en kernels auditados.
