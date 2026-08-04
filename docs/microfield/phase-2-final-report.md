# Informe final de Fase 2

Fecha de cierre técnico: 2 de agosto de 2026.

## Resultado ejecutivo

Microfield ya permite que un consumidor declare un campo binario GF(2^m), lo
certifique y genere antes de compilar un tipo Rust nominal, sin editar la
librería y sin pagar por extensibilidad en el camino escalar. El mismo tipo
dispone de batch portable y, cuando su perfil estructural lo permite, adapters
ISA propiedad del runtime.

La Fase 2 añade PCLMUL, PMULL, VPCLMUL, batches persistentes y selección previa
sin exponer limbs, intrinsics, punteros ni catálogos. El cierre es
deliberadamente conservador: solo PCLMUL participa automáticamente; PMULL y
VPCLMUL permanecen forzables hasta que una calibración diversa supere los gates
publicados.

## Entregas por hito

| Hito | Entrega cerrada |
|---|---|
| H2.1 | `BinaryFieldFactory`, Builder, Rabin, typestate y codegen externo |
| H2.2 | optimizador portable estático para sparse, dense y low-tail |
| H2.3 | capabilities no falsificables y selector inmutable |
| H2.4 | PCLMUL x86-64 para presets y puente genérico posterior |
| Puente ABI 3 | perfiles ISA certificados para campos externos |
| H2.5 | PMULL AArch64 en presets y ABI 3 |
| H2.6 | `PackingPlan`, `PackedBatch` y vistas sin `alloc` |
| H2.7 | VPCLMUL por pares y `AosLanePairs` |
| H2.8 | calibración versionada, hardening, ABI estable e informe final |

## Extensibilidad conseguida

`BinaryFieldFactory` acepta característica dos, base polinómica, grados
2..=4096, módulo mónico irreducible y encoding little/lsb0. Manifiesto y
Builder convergen en la misma normalización. El resultado contiene:

- newtype nominal y representación privada;
- traits escalares y operadores estándar;
- reducción completa para módulos multilimb;
- cuadrado dedicado e inversión Itoh–Tsujii;
- `FieldId`, `ArtifactId`, certificado y planes;
- batch portable seguro;
- perfil ISA estructural ABI 3.

Los fixtures externos ejercitan grados 9, 10, 128, 192 y 233, incluidos
módulos sparse, dense y low-tail. Compilan además scalar-only `no_std` sin
arrastrar el generador al runtime.

## Política de ejecución cerrada

| Backend | Presets | Campos ABI 3 | Selección automática v1 |
|---|---|---|---|
| Portable | sí | sí | fallback universal |
| x86 PCLMUL | sí, Karatsuba | sí, schoolbook verificado | sí en presets desde 1 |
| x86 VPCLMUL | sí, pares | sí, pares schoolbook | no; explícito |
| AArch64 PMULL | sí, Karatsuba | sí, schoolbook verificado | no; explícito |

`CpuCapabilities::detect()` toma una única instantánea. `EngineBuilder` valida
build, campo, CPU, política y threshold; `Engine` conserva un `KernelSet<F>`
inmutable. No se detecta ni selecciona dentro de una operación.

La tabla autoritativa está en
`crates/microfield/calibration/selection-table-v1.csv`. Se transforma en
constantes privadas, no en un registro runtime.

## Rendimiento y decisión H2.8

La evidencia Intel i7-13700HX confirma PCLMUL con mejora conservadora superior
al 20 % desde un elemento en los tres presets. El overhead de la fachada queda
por debajo del 3 % en la región grande.

VPCLMUL mejora el pipeline GF(2¹²⁸) solo alrededor de 3–7 % en lotes medianos
y grandes de esa CPU, y pierde aproximadamente 36–38 % en GF(2²⁵⁶). PMULL es
correcto sobre hardware ARM64 real, pero todavía no tiene dos perfiles de
rendimiento normalizados. Ninguno entra por ello en `Auto`.

`capture_calibration.sh` conserva comando, CPU, familia, microcode, SO, kernel,
Rust, LLVM, estimadores Criterion y SHA-256. El workflow manual ejecuta la
misma captura sobre runners x86-64 y ARM64. La CI ordinaria valida la tabla y
los perfiles versionados, no tiempos de pared ruidosos.

## Seguridad

El crate niega `unsafe` globalmente. Las únicas excepciones son:

- adapter x86 PCLMUL;
- adapter x86 VPCLMUL;
- adapter AArch64 PMULL;
- ownership de storage alineado.

El inventario v1 autentica los cuatro archivos completos. Los adapters reciben
valores Rust válidos por valor y solo se alcanzan después de comprobar target
features. Miri cubre portable, storage y codegen; ASan cubre las tres fronteras
ISA; canarios, tails, in-place y errores transaccionales completan la matriz.

Los audits de ensamblado exigen la instrucción esperada y rechazan asignador,
división, dispatch indirecto y crecimiento por encima de un presupuesto
estructural amplio. No se fija una secuencia de bytes dependiente de LLVM.

## Reproducibilidad

- manifests y artefactos se regeneran con diff vacío;
- Sage usa vectores v2 tipados y seed autenticada;
- el corpus diferencial v1 fija 20 tamaños/seeds, incluidos límites de tile y
  thresholds;
- un fallo informa el primer índice y la longitud mínima reproducible;
- cada perfil Criterion usa schema v1 y entorno completo;
- ningún benchmark de QEMU se acepta como evidencia de selección.

## Compatibilidad e identidades

El runtime `0.1.x` acepta codegen ABI 1..=3 y el generador emite ABI 3. La
versión emitida procede de `CURRENT_CODEGEN_ABI_VERSION`, utilizada tanto por
la API del paquete generado como por el `const` de compatibilidad.

Los tres presets pasan por la factory pública durante los tests y deben
reproducir `FieldId`, `ArtifactId` y los ocho artefactos byte a byte.

- Renombrar conserva `FieldId` y `ArtifactId`, pero cambia bundle y fuente.
- Cambiar build, IR, optimizador, codegen o perfil conserva `FieldId` y cambia
  `ArtifactId`.
- Cambiar matemática o encoding cambia `FieldId`.

## Gates de salida

- pruebas de todos los targets y consumidor externo;
- leyes algebraicas, referencia lenta y vectores Sage;
- corpus diferencial en toda ISA disponible;
- feature matrix, `no_std` y cross-compilation AArch64;
- MSRV 1.89;
- Clippy y rustdoc sin warnings;
- Miri y AddressSanitizer;
- cero asignaciones en batch reutilizado;
- artefactos deterministas;
- auditoría ASM e inventario `unsafe`;
- compatibilidad con las 447 pruebas de la biblioteca legada.

## Limitaciones deliberadas

- solo GF(2^m) en base polinómica;
- generación estática, no campos heterogéneos runtime;
- PMULL y VPCLMUL requieren selección explícita;
- sin AVX-512, SVE, RISC-V, paralelismo interno ni autotuning;
- sin garantía de durabilidad ante caída o escritores concurrentes para la
  publicación de artefactos;
- benchmarks legados ajenos a Microfield permanecen fuera del gate aislado.

## Entrada a Fase 3

Fase 3 puede construir algoritmos de nivel superior sin reabrir identidad,
encoding o selección ISA. El orden recomendado es:

1. inversión batch mediante Montgomery trick con workspace explícito;
2. evaluación Horner y productos batch especializados;
3. cadenas de inversión/potencia generadas por campo;
4. perfiles de timing y APIs de secreto/público;
5. solo después, nuevas familias matemáticas como GF(p) impar.

La calibración de una CPU adicional puede promover backends dentro de la tabla
versionada, pero no bloquea este trabajo ni cambia la API.
