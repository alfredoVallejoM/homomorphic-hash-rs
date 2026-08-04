# ADR 0017 — VPCLMUL por pares de lanes y selección conservadora

- Estado: aceptado e implementado
- Fecha: 2 de agosto de 2026
- Hito: H2.7

## Contexto

`VPCLMULQDQ` puede ejecutar un producto carry-less independiente en cada lane
de 128 bits de un registro AVX2. La presencia de la instrucción no demuestra
por sí sola una mejora: el coste de formar lanes, reducir dos productos, tratar
tails y mover datos puede superar a dos kernels PCLMUL escalares.

H2.7 debía aportar una implementación verificable sin modificar el layout de
los elementos ni introducir AVX-512, y solo podía entrar en selección
automática con evidencia del pipeline completo.

## Decisión

Se compila `backend::x86_vpclmul` en x86-64 y se exige conjuntamente
`pclmulqdq`, `avx2` y `vpclmulqdq`. El selector realiza esa comprobación una vez
antes de entregar sus funciones seguras.

Los presets usan Karatsuba especializado. Cada registro contiene dos elementos
GF(2¹²⁸) independientes; GF(2²⁵⁶) aplica un nivel exterior adicional. Los
perfiles externos ABI 3 usan el mismo puente verificado y un producto
schoolbook por pares, seguido de la reducción generada propia de cada campo.
El código generado no contiene intrinsics, `unsafe` ni punteros de función.

`PackedLayout::AosLanePairs` conserva elementos AoS, pero agrupa dos elementos
consecutivos como una tesela de ejecución. `PackingPlan` garantiza:

- alineación inicial de 32 bytes;
- longitud padded par;
- padding completamente inicializado a cero;
- identidad de backend y campo;
- ausencia de exposición de limbs o lanes.

La API ordinaria sobre slices acepta cualquier longitud. Una cola impar se
ejecuta emparejando el último elemento con cero y descartando la segunda lane.
La API packed elimina esa cola mediante padding. Las rutas in-place leen la
pareja completa antes de escribir. Cada operación aritmética termina con un
solo `vzeroupper` fuera del bucle.

Los tres presets y todos los perfiles externos quedan forzables, pero
`automatic_selection = false`. La metadata conserva una región candidata local
de 64 elementos para GF(2¹²⁸); los dos campos de 256 bits publican
`minimum_batch = usize::MAX`, porque no apareció un cruce favorable. Estos
valores no restringen longitudes correctas y no son una garantía para otra CPU.

## Evidencia local y decisión de selección

Medición Criterion del 2 de agosto de 2026: Intel Core i7-13700HX, microcode
`0x12f`, Linux 6.18.7, rustc 1.96.0-nightly/LLVM 22.1.0, perfil `bench`, 20–30
muestras y 1 segundo de warm-up/medición.

| Campo/lote/región | PCLMUL | VPCLMUL | Resultado |
|---|---:|---:|---:|
| GF(2¹²⁸)/8/kernel | 26,803–26,832 ns | 27,230–27,315 ns | VPCLMUL pierde |
| GF(2¹²⁸)/64/kernel | 206,27–210,00 ns | 201,33–201,52 ns | mejora pequeña |
| GF(2¹²⁸)/64/pipeline reutilizado | 233,52–234,87 ns | 225,92–226,97 ns | mejora ≈3–4 % |
| GF(2¹²⁸)/4096/kernel | 12,833–12,842 µs | 12,449–12,553 µs | mejora ≈2–3 % |
| GF(2¹²⁸)/4096/pipeline reutilizado | 16,079–16,573 µs | 15,435–15,454 µs | mejora ≈4–7 % |
| HH-256/4096/kernel | 38,664–38,732 µs | 52,694–52,948 µs | VPCLMUL pierde ≈36 % |
| Alt-256/4096/kernel | 39,260–39,430 µs | 54,064–54,395 µs | VPCLMUL pierde ≈38 % |

Aunque GF(2¹²⁸) gana en lotes medianos/grandes, el margen procede de una única
microarquitectura y no justifica una regla universal basada solo en feature
bits. H2.7 queda por tanto correcto, medido y documentadamente fuera de
`Auto`/`Throughput`. Un backend más nuevo no desplaza a PCLMUL si empeora la
ruta medida.

## Frontera de seguridad

El crate mantiene `#![deny(unsafe_code)]`. Solo `backend::x86_vpclmul` recibe
una excepción de módulo, junto a PCLMUL, PMULL y el storage packed ya auditados.
No se realizan cargas desalineadas mediante punteros de usuario: los valores se
extraen por valor desde tipos nominales válidos.

La auditoría de ELF exige `vpclmul*`, `vzeroupper`, ausencia de llamadas
indirectas internas y ausencia del asignador. ASan ejecuta la suite diferencial
VPCLMUL y las rutas packed; el test estructural rechaza cualquier expansión no
declarada de `unsafe`.

## Consecuencias

- Los usuarios pueden forzar VPCLMUL para pruebas o despliegues calibrados.
- Los campos ABI 3 adquieren VPCLMUL sin editar Microfield ni emitir código ISA.
- `Auto`, `LowLatency` y `Throughput` conservan PCLMUL en x86-64 por ahora.
- El nuevo layout no cambia tamaño, alineamiento natural ni encoding de `F`.
- El `ArtifactId` de los presets cambia porque el conjunto autenticado de
  backends del artefacto ahora incluye VPCLMUL; `FieldId` permanece idéntico.
- H2.8 deberá recoger medidas en varias microarquitecturas antes de versionar
  una tabla de selección más específica.

## Alternativas rechazadas

- Activarlo por presencia de CPU: confunde disponibilidad con rendimiento.
- Seleccionar VPCLMUL para 256 bits: las medidas locales muestran una regresión
  muy superior al gate del 3 %.
- Exponer SoA de limbs: rompe encapsulación y estabiliza representación interna.
- Repacking implícito por operación: oculta el coste y elimina amortización.
- AVX-512: queda fuera del alcance y cambiaría frecuencia, ABI y matriz de CI.
