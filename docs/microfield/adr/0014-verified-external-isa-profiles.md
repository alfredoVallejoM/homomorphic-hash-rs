# ADR 0014: perfiles ISA verificados para campos externos

- Estado: aceptado.
- Fecha: 2026-08-01.
- Hito: puente entre H2.4 y H2.5.

## Contexto

ABI de codegen 1/2 permitía generar cualquier campo binario válido del esquema
v1, pero solo podía asociarlo al backend portable. Abrir `KernelSet`, punteros
de función o constructores de catálogos habría permitido saltarse la detección
de CPU y ampliar el límite `unsafe` a cada consumidor.

También era necesario separar dos afirmaciones distintas:

1. una representación y su reducción son compatibles con un producto
   carry-less por limbs de 64 bits;
2. ese producto es más rápido en una CPU y región de tamaños concretas.

La primera se demuestra durante generación. La segunda exige mediciones en
hardware representativo y no puede aceptarse desde un manifiesto.

## Decisión

ABI de codegen 3 genera un `VerifiedIsaProfile` exclusivamente después de:

1. normalización estricta del manifiesto;
2. prueba de irreducibilidad de Rabin;
3. construcción del producto ancho y del plan de reducción;
4. selección portable determinista;
5. serialización canónica y digest con dominio
   `microfield:verified-isa-profile:v1`.

El perfil contiene `FieldId`, clase de grado, layout polinómico little-endian,
limbs de entrada/producto, digest del plan de reducción, backends compatibles,
calendario completo (`fixed` o `data_dependent`) y estado de selección. Se
publica como `verified-isa-profile.json`, forma parte del IR v3 y de
`ArtifactId`, y su digest se incrusta en la fuente generada. El bundle autentica
certificado, perfil, plan y fuente como una unidad.

La fuente ABI 3 implementa el contrato oculto y seguro
`VerifiedBinaryIsaField<LIMBS, WIDE_LIMBS>`. Solo entrega arrays por valor y una
reducción segura. `VerifiedIsaStrategy` construye dentro del runtime las tablas
x86 o AArch64; desde H2.7 incluye PCLMUL, VPCLMUL y PMULL. El consumidor no
entrega intrinsics, funciones ni metadatos de CPU. `KernelCatalog` y `KernelSet`
siguen sin constructor público raw.

Los perfiles externos nacen como `explicit_only`. Un backend forzado puede
usarlos tras detección confiable, pero `Auto`, `LowLatency`, `Throughput` y
`FixedSchedule` no los seleccionan sin una calibración posterior. El campo
portable permanece siempre disponible.

## Alcance

El puente cubre todos los campos aceptados por el esquema binario v1, grados
2..=4096, no solo los tres presets ni las potencias de dos. Las potencias de
dos alineadas conservan una clase explícita y la reducción low-tail; también se
prueban reducción sparse, dense y un grado 233 no alineado.

Editar manualmente la fuente generada puede falsear sus constantes matemáticas,
igual que en ABI anteriores. La autenticidad se verifica mediante los digests
del paquete, no mediante un secreto embebido en el runtime. Incluso una
implementación manual del contrato oculto no puede provocar ejecución ISA sin
capabilities ni introducir `unsafe` en Microfield: solo podría construir un
tipo matemáticamente incorrecto dentro de su propio crate.

## Consecuencias

- `FieldId` no cambia: describe el campo y encoding.
- `ArtifactId` sí cambia: ahora autentica IR v3 y el perfil.
- el runtime conserva compatibilidad con fuente ABI 1, 2 y 3;
- la fuente nueva sigue siendo `no_std`, sin `unsafe`, heap o dispatch escalar;
- el producto genérico schoolbook tiene calendario fijo; el perfil completo
  publica `fixed` para low-tail y `data_dependent` para sparse/dense;
- los presets pueden conservar Karatsuba especializado;
- corrección certificada no se transforma automáticamente en un claim de
  rendimiento.

## Evidencia

El fixture externo genera grados 9, 10 denso, 128, 192 low-tail y 233. En x86 y
AArch64 compara producto, cuadrado e in-place ISA contra portable para las tres
clases estructurales y tres familias de reducción; x86 cubre PCLMUL y VPCLMUL.
Los digests se
recalculan de forma independiente; el módulo de grado 192
\(x^{192}+x^7+x^2+x+1\) fue confirmado irreducible tanto por Rabin como por
Sage. La regeneración de los tres bundles mantenidos produce diff vacío.
