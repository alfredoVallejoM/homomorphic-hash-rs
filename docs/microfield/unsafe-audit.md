# Auditoría de `unsafe` de Fases 2–4

## Resultado

Fase 4 conserva `#![deny(unsafe_code)]` en la raíz y cinco excepciones
locales. No existe `unsafe` en álgebra portable, encoding, identidad, selector,
factory, codegen ni API pública.

| Frontera | Motivo | Precondición | Evidencia |
|---|---|---|---|
| `backend/x86_pclmul.rs` | intrinsics PCLMUL | snapshot real con `pclmulqdq` | diferencial, ASan y ELF |
| `backend/x86_vpclmul.rs` | intrinsics AVX2/VPCLMUL | `pclmulqdq + avx2 + vpclmulqdq` | diferencial, ASan, tails y ELF |
| `backend/aarch64_pmull.rs` | intrinsic PMULL | snapshot real con NEON + PMULL | diferencial ARM64, ASan y ASM |
| `backend/x86_prime.rs` | AVX2/BMI2 para campos primos | snapshot real con AVX2 o BMI2 | diferencial, ASan, tails y ASM |
| `engine/packed/storage.rs` | asignación alineada y slices tipados | layout probado, ownership único e inicialización total | Miri, ASan, canarios y vistas |

Los wrappers ISA reciben slices de elementos Rust válidos y no exponen
punteros ni representaciones vectoriales. Las cargas SIMD se acotan por tiles;
el resto se procesa escalarmente. Solo son alcanzables después de que
`EngineBuilder` valide arquitectura y features. El storage inicializa también
el padding antes de construir cualquier slice.

## Gate de revisión

`unsafe/unsafe-inventory-v2.sha256` fija el hash de las cinco implementaciones
revisadas. `tools/audit_unsafe_scope.sh` falla cuando:

- aparece una sexta frontera;
- cambia cualquiera de los cinco archivos sin actualizar el inventario;
- cambia el número de excepciones al lint;
- falta documentación `SAFETY` en una frontera.

Actualizar un hash no es un arreglo mecánico: exige revisar precondiciones,
aliasing, inicialización, alineamiento, target features, unwinding y ownership;
después deben repetirse Miri, ASan, diferencial y auditoría de ensamblado según
la frontera afectada.

## Resultado de los revisores dinámicos

- Miri: portable completo, campos primos, almacenamiento owned y rutas ABI 3.
- ASan x86-64: PCLMUL, VPCLMUL, AVX2/BMI2 primo, packed y campos externos.
- ASan AArch64: PMULL y packed sobre hardware nativo.
- Los tests estructurales rechazan expansión del alcance y los scripts ISA
  rechazan asignador, división y dispatch indirecto en kernels auditados.
