# ADR 0015: backend batch AArch64 PMULL

- Estado: aceptado con selección automática pendiente de calibración.
- Fecha: 2026-08-01.
- Hito: H2.5.

## Contexto

H2.3 ya detectaba NEON y PMULL, y H2.4 fijó la frontera usada por PCLMUL. Faltaba
una estrategia AArch64 con la misma semántica, sin cambiar tipos, encoding,
`FieldId`, ruta escalar o ABI batch.

## Decisión

El adaptador `backend::aarch64_pmull` usa `vmull_p64` tras una frontera
`#[target_feature(enable = "neon,aes")]`. Rust denomina `aes` al feature de
compilación que incluye PMULL; la selección runtime exige por separado NEON y
`is_aarch64_feature_detected!("pmull")`.

Para los presets:

- GF(2¹²⁸) usa Karatsuba con tres PMULL;
- GF(2²⁵⁶) usa un nivel exterior, nueve PMULL;
- el cuadrado dedicado usa dos o cuatro PMULL sin términos cruzados;
- la reducción reutiliza los reductores low-tail certificados;
- batch out-of-place, tails e in-place operan sobre el alineamiento natural de
  8 bytes, sin packing, scratch ni asignaciones.

Los perfiles externos ABI 3 usan schoolbook monomorfizado para cualquier
número fijo de limbs y delegan su reducción al plan generado. Ningún intrinsic
cruza esa frontera segura.

PMULL queda registrado como `explicit_only`, incluso para los presets, hasta
disponer de mediciones reproducibles en hardware ARM real. Puede seleccionarse
con `force_backend(Aarch64Pmull)` después de `detect()`, también bajo
`FixedSchedule`; las políticas automáticas conservan portable. QEMU solo se
usa para corrección, nunca para decidir umbrales.

## Seguridad

El crate mantiene `#![deny(unsafe_code)]`. Los dos adaptadores ISA y, desde
H2.6, el módulo independiente de storage alineado tienen excepción local; una
prueba estructural rechaza cualquier cuarto sitio. Los
wrappers seguros solo son alcanzables mediante catálogos y el selector valida
backend compilado, campo, CPU y política antes de entregar `Engine`.

No se usan cargas vectoriales sobre el elemento, punteros aportados por el
usuario ni layouts SIMD persistentes. `vmull_p64` recibe limbs por valor y
devuelve el producto de 128 bits, que se descompone de forma endian explícita.

## Evidencia

- cross-check y Clippy AArch64 para `std` y `no_std`;
- ensamblado release con PMULL, especializaciones 128/256, sin `br`/`blr` ni
  referencias al asignador;
- QEMU 8.2 `-cpu max`: tres pruebas públicas, tres campos, todos los bits de
  base y 17 tamaños hasta 16 384;
- QEMU + ASan: la misma suite PMULL;
- fixture ABI 3 bajo QEMU y QEMU + ASan: 11 pruebas, cinco grados, tres clases
  y tres reducciones;
- job CI nativa `ubuntu-24.04-arm` para repetir diferencial, perfiles externos,
  packed batches, cero asignaciones, Clippy, ensamblado y ASan en hardware real.

## Validación sin hardware ARM local

La ausencia de PMULL en la máquina de desarrollo no impide implementar ni
publicar el backend, pero obliga a separar cuatro clases de evidencia:

1. **Compilación cruzada:** el target `aarch64-unknown-linux-gnu` demuestra que
   API, cfgs, intrinsics y features son aceptados por Rust para AArch64.
2. **Auditoría estática:** el script release verifica instrucciones PMULL,
   especializaciones, ausencia de calls indirectas y referencias al asignador.
3. **Ejecución emulada:** QEMU `-cpu max` ejecuta pruebas diferenciales y ASan;
   valida semántica, tails e integridad de memoria, no latencia ni throughput.
4. **Hardware remoto:** el job ARM64 de cada push es el gate final de ejecución
   real. Si este job no está verde, el commit no se considera validado para
   publicar PMULL.

Opcionalmente puede repetirse el cuarto gate en un runner self-hosted, una VM
ARM64 con extensión crypto o proveedores AArch64 distintos para ampliar
diversidad. Ninguna de esas máquinas cambia el contrato: el selector sigue
exigiendo detección runtime real.

La versión inicial puede lanzarse con PMULL forzable porque corrección y
seguridad tienen gate en hardware remoto. La selección automática permanece
deshabilitada hasta medir en al menos hardware ARM representativo, comparar
contra portable con intervalos reproducibles y fijar thresholds por campo.

## Consecuencias

- H2.5 entrega corrección y una ruta de uso explícita sin afirmar rendimiento;
- la calibración nativa podrá cambiar metadata/umbral sin modificar campo,
  encoding o API;
- PMULL2 no se introduce: no aporta una frontera distinta y deberá demostrar
  una mejora antes de ampliar el adaptador;
- H2.6 puede depender de una base PCLMUL/PMULL común ya auditada.
