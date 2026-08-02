# Informe de la extensión F4.6-SIMD

Fecha: 2 de agosto de 2026.

## Resultado

La cobertura SIMD prima deja de estar limitada a `Fp251V1`. Goldilocks dispone
de un backend AVX2 mantenido y rentable; los campos externos canónicos de 8 y
16 bits pueden obtener candidatos AVX2 seguros mediante factories estáticos; y
VPCLMUL procesa dos pares independientes por iteración para aumentar ILP. La
API pública de los elementos, `Engine<F>` y el ABI neutral de `KernelSet` no
cambian.

## Implementación

### Primos canónicos pequeños

Los puertos público-ocultos `VerifiedPrimeCanonical8Field` y
`VerifiedPrimeCanonical16Field` separan el perfil certificado del adapter ISA.
El tipo consumidor expone valores por copia y constantes verificables; el
runtime crea el único `KernelSet` posible.

- `u8`: 32 residuos por tesela, widening a `u16`, Barrett con `mulhi` y
  compactación canónica;
- `u16`: 16 residuos por tesela, widening a `u32`, high-half unsigned de la
  recíproca Barrett y compactación;
- suma, producto, square e in-place usan funciones monomorfizadas;
- el tail es escalar y no asigna.

La batería externa define `Fp17` y `Fp65521` fuera del crate. Fp17 recorre los
289 pares posibles; Fp65521 combina fronteras, casos adversarios, muestras
sembradas y tamaños 0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255 y 1024.
También verifica cero asignaciones y que `Auto` no elige candidatos externos.
El reducer vectorial se prueba además directamente y de forma exhaustiva para
`p = 3, 17, 251` y `p = 257`; módulos intermedios `769`, `4093` y `65521`
cubren fronteras y miles de teselas sembradas.

### Goldilocks AVX2

El nuevo backend trabaja en cuatro lanes `u64`:

1. descompone cada producto 64x64 en cuatro `vpmuludq` de 32 bits;
2. reconstruye las mitades de 128 bits con máscaras de carry unsigned;
3. ejecuta cuatro folds Solinas de iteración fija;
4. aplica comparación unsigned mediante bias del bit de signo y corrección
   branchless;
5. procesa el resto con la misma aritmética escalar certificada.

El test interno cruza todos los pares de bits de base 64x64, fronteras y 25.000
vectores sembrados de cuatro lanes contra Barrett escalar. La integración cubre
suma, producto, square, in-place, tails y selección por tamaño.

### VPCLMUL

Los kernels mantenidos y el bridge ABI 3 ejecutan ahora dos operaciones YMM
independientes —cuatro elementos— antes del tail de pareja. Esto aumenta el
paralelismo a nivel de instrucción sin crear un layout nuevo ni ampliar
`unsafe`. La política no cambia: sigue siendo explícito porque no supera a
PCLMUL de forma estable en toda su región.

## Calibración local

Entorno: Intel Core i7-13700HX, x86-64, Rust nightly 1.96.0
`1d8897a4e`, LLVM 22.1.0. Los intervalos Criterion completos de la frontera de
selección se conservan en
[`phase46-simd-i7-13700hx-2026-08-02.csv`](../../crates/microfield/calibration/phase46-simd-i7-13700hx-2026-08-02.csv).

| Operación Goldilocks, 4 elementos | Portable | AVX2 | Decisión |
|---|---:|---:|---|
| producto | 10,407–10,437 ns | 6,8671–6,8748 ns | promover |
| square | 9,6481–9,8514 ns | 6,7253–6,7427 ns | promover |
| suma | 4,3101–4,3996 ns | 4,3095–4,3383 ns | sin regresión |

Producto AVX2 mantuvo una mejora aproximada del 25–33 % entre 4 y 16.384
elementos. Por ello `minimum_batch = 4` y `preferred_multiple = 4` son parte de
la metadata mantenida.

El desenrollado VPCLMUL fue neutral dentro del ruido para GF(2^128), mejoró
aproximadamente 3,4 % GF(2^256)-HH/4096 y 8,9 % GF(2^256)-Alt/4096. No basta
para reemplazar PCLMUL automáticamente. La generalización AVX2 independiente
del layout para `Fp251V1` fue correcta pero cerca de 8x más lenta en 64
elementos que su implementación zero-copy; se conserva la especializada.

Estas cifras justifican decisiones del artefacto medido, no prometen el mismo
porcentaje en otra microarquitectura.

## Seguridad y auditabilidad

La frontera `unsafe` continúa limitada a cinco archivos. No se abre un sexto:
Goldilocks y los puentes pequeños viven en `backend/x86_prime.rs`, mientras el
desenrollado permanece en `backend/x86_vpclmul.rs`. El inventario SHA-256 se
actualiza después de revisar precondiciones, bounds y target features.

La auditoría x86 prima exige ahora, además de la evidencia anterior,
`vpmuludq`, `vpcmpgtq`, shifts de 64 bits, `vzeroupper` y ausencia de división,
asignador o dispatch indirecto. ASan ejecuta también ambos campos externos.

## Garantías y límites

- Goldilocks AVX2 es automático desde cuatro elementos en CPUs con AVX2.
- Fp251 AVX2 especializado sigue automático desde 64 elementos.
- Los puentes externos `u8`/`u16`, BMI2 y VPCLMUL son explícitos.
- SIMD no se deduce solo del número de bits: representación y reducción deben
  coincidir con un perfil verificado.
- AArch64 primo e IFMA siguen pendientes de hardware, implementación y
  calibración real.
- El puente externo actual copia a teselas locales. Una representación runtime
  mantenida o packed persistente será necesaria para una ruta externa
  sin repacking entre operaciones.

Con estas condiciones, F4.6-SIMD queda cerrada localmente. El siguiente corte
es [`F4.7-PACKED-SIMD`](phase-4-7-plan.md), que implementará el storage por
lanes antes de que Fase 5 genere perfiles primos externos completos.
