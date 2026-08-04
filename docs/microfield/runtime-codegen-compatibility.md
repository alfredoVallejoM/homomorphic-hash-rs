# Compatibilidad runtime/codegen

## Matriz congelada al cerrar Fase 2

| Runtime | ABI mínimo aceptado | ABI máximo aceptado | ABI emitido | Esquema manifiesto | Esquema artefacto |
|---|---:|---:|---:|---:|---:|
| `0.1.x` | 1 | 3 | 3 | 1 | 1 |

La forma autoritativa y comprobable por tests está en
`crates/microfield/abi/runtime-codegen-matrix-v1.csv`.

ABI 1 conserva los helpers portables generales. ABI 2 añade las rutas
monomorfizadas del optimizador portable. ABI 3 añade el perfil ISA verificado,
pero no permite que el código generado aporte intrinsics, punteros de función
o capabilities falsificables.

## Reglas de evolución

1. El generador emite `CURRENT_CODEGEN_ABI_VERSION` y el runtime comprueba el
   rango en un `const` del módulo consumidor.
2. Añadir ABI `N+1` exige que el runtime acepte al menos `N` y `N+1` antes de
   que el generador empiece a emitirlo.
3. Retirar ABI `N-1` requiere una revisión incompatible explícita; nunca se
   hace dentro de una actualización compatible de `0.1.x`.
4. Los símbolos bajo `__private` son un contrato exclusivamente entre el
   generador certificado y el runtime. No constituyen API manual estable.
5. Una fuente con ABI fuera del rango falla al compilar, no durante una
   operación de campo.

## Identidades

- `FieldId` autentica matemática y encoding. Excluye nombre, build y backend.
- `ArtifactId` añade generador, IR, build, optimizador y perfil ISA verificado.
- `ArtifactBundleDigest` autentica rutas y bytes exactos del bundle.
- `GeneratedFieldPackage::package_digest` añade los bytes exactos del módulo
  Rust externo.

Renombrar una presentación conserva `FieldId` y `ArtifactId`, pero cambia el
bundle y el paquete Rust. Cambiar un plan, build, codegen o perfil verificado
conserva `FieldId` y cambia `ArtifactId`. Los tests H2.8 congelan ambas reglas.

## Regeneración de presets

Los tres manifiestos mantenidos se introducen también por
`BinaryFieldFactory::from_manifest`. El gate compara `FieldId`, `ArtifactId` y
cada byte de sus ocho artefactos con el directorio versionado. Así no existe un
camino privilegiado distinto para presets y consumidores externos.
