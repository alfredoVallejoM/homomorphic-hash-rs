---
title: "Especificación funcional de Microfield"
subtitle: "Fases 0, 1 y 2: núcleo portable, factory binaria y backends x86-64/AArch64"
author: "Plan de implementación derivado de Arquitectura desde primeros principios"
date: "2 de agosto de 2026"
lang: es-ES
status: "fase-1-y-fase-2-cerradas"
---

# Resumen ejecutivo

> **Revisión SOLID v1.** Este documento se aplica junto con los ADR y la
> documentación de `docs/microfield/`. Ante una contradicción, prevalecen los
> ADR más recientes. La revisión introduce traits segregados, un contrato
> neutral de kernels, puertos y adaptadores para el generador y una política
> explícita de abstracciones de coste cero.

> **Estado de implementación, 2 de agosto de 2026.** La Fase 1 está cerrada en
> `main` mediante `95f82f5`. El esquema ejecutable v1 permanece limitado a
> GF(2) en base polinómica con encoding `little`/`lsb0`. La Fase 2, H2.1–H2.8,
> está cerrada: factory estática, optimizador, selector, perfiles ABI 3,
> PCLMUL, PMULL, packed batches, VPCLMUL y contratos de calibración/seguridad.
> PCLMUL es automático; PMULL y VPCLMUL permanecen explícitos hasta superar el
> gate multi-familia. `Prime`, `Normal`, `Tower` y contextos dinámicos
> permanecen fuera del alcance.

Este documento convierte `arquitectura_campos_finitos_vectorizados` en una
especificación funcional implementable para sus tres fases iniciales:

- **Fase 0:** especificación, identidad, validación, generación y armazón de
  pruebas;
- **Fase 1:** vertical binario portable completo;
- **Fase 2:** factory estática de campos binarios externos, kernels x86-64 y
  AArch64, selección de backend y layouts batch.

La decisión principal es reducir la fragmentación de la propuesta original.
Durante estas fases habrá **un solo paquete Cargo llamado `microfield`**, con:

- un target de biblioteca;
- un binario integrado `microfield-gen`;
- módulos internos jerárquicos;
- presets generados dentro del paquete y tipos externos emitidos en el crate
  consumidor;
- tests, benchmarks, manifiestos y certificados en el mismo repositorio.

No habrá un crate por cada arquitectura, familia de campo o responsabilidad.
La separación seguirá existiendo como **fronteras internas de dependencia**,
porque mezclar especificación, álgebra y `unsafe` impediría comprobar la
corrección. La estructura compacta no significa un archivo monolítico.

Rust no ofrece herencia de implementación entre clases. El mecanismo que
cumple ese objetivo con coste cero es:

1. jerarquía de traits por capacidades;
2. composición entre estructuras;
3. tipos de estado para que solo progresen especificaciones válidas;
4. tipos concretos generados;
5. funciones genéricas monomorfizadas;
6. tablas de kernels seleccionadas una sola vez para operaciones batch.

El elemento de campo nunca contendrá un backend ni un puntero de función. Por
ejemplo:

```rust
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Gf2_256HhV1([u64; 4]);
```

Este mismo valor podrá ser operado por el portable, PCLMUL, VPCLMUL o PMULL.
La semántica se mantiene estable y solamente cambia el ejecutor.

Las tres fases producirán un núcleo útil, pero todavía no implementarán
firmas algebraicas, campos primos, contextos dinámicos, grafos ni
canonización. La salida será:

$$
\text{manifiesto}
\longrightarrow
\text{campo validado}
\longrightarrow
\text{tipo generado}
\longrightarrow
\text{portable}
\longrightarrow
\text{Engine con x86/ARM}.
$$

```mermaid
flowchart LR
    Manifest[Manifiesto] --> Validated[Campo validado]
    Validated --> Generated[Tipo generado]
    Generated --> Portable[Portable]
    Portable --> Engine[Engine batch]
    Engine --> Native[Backends ISA]
```

# 1. Decisiones normativas

## 1.1 Unidad de distribución

**ARCH-001.** Las Fases 0-2 de Microfield se desarrollarán en un único paquete
Cargo llamado `microfield`. El repositorio será un workspace que conservará
temporalmente el paquete legado `homomorphic-hash-rs` durante la migración:

```text
workspace/
  Cargo.toml
  src/                    # paquete legado
  crates/
    microfield/
      Cargo.toml
      src/
      fields/
      artifacts/
      tests/
      benches/
      certificates/
      reference-vectors/
```

El paquete tendrá:

```toml
[lib]
name = "microfield"

[[bin]]
name = "microfield-gen"
required-features = ["generator"]
```

El binario puede depender de la biblioteca del mismo paquete. No necesita un
crate `xtask`, otro workspace ni una proc macro.

## 1.2 Fronteras internas

**ARCH-002.** Existirán dominios internos con dependencias unidireccionales:

| Dominio | Responsabilidad | Puede depender de |
|---|---|---|
| `field` | contratos algebraicos, encoding y metadatos | `error`, `id` |
| `binary` | algoritmos portables y referencias | `field` |
| `kernel` | ABI neutral y catálogos estáticos | `field` |
| `backend` | estrategias ejecutoras | `field`, `binary`, `kernel` |
| `engine` | selección y fachada batch | `field`, `kernel` |
| `spec` | modelos, casos de uso y puertos del generador | utilidades puras |
| `packed` | buffers y layouts persistentes de Fase 2 | `field`, `engine` |

La regla de runtime es:

$$
\texttt{field}
\to
\{\texttt{binary},\texttt{kernel}\}
\to
\{\texttt{backend},\texttt{engine}\}
\to
\texttt{packed}.
$$

`engine` nunca importa un backend concreto: recibe su catálogo desde el tipo de
campo generado. Los backends no podrán reintroducir decisiones matemáticas. Una reducción
es definida por un plan generado; x86 y ARM solo la bajan a instrucciones.

## 1.3 Herencia sustituida por traits

**ARCH-003.** La jerarquía pública será:

```text
Field
  |-- CanonicalEncoding
  |-- ExtensionField
        \-- BinaryPolynomialField
  \-- capacidades futuras
        |-- PrimeField
        \-- FftField
```

No se creará un trait universal con `shift_phase`, FFT, Montgomery o acceso a
limbs. Una aplicación genérica declarará exactamente la capacidad que necesita.

**ARCH-004.** Los campos incluidos en Fases 1-2 compartirán implementación
mediante código generado y traits internos. No compartirán representación por
herencia.

**ARCH-004.1.** La extensibilidad externa usa generación estática en `build.rs`.
La factory produce un tipo nominal antes de compilar; no devuelve contextos de
campo ni elementos dinámicos en runtime. Su salida portable no puede registrar
catálogos raw o afirmar compatibilidad ISA.

## 1.4 Dispatch

**ARCH-005.** Las operaciones escalares:

```rust
let c = a * b;
```

serán estáticas y portables. No habrá llamada indirecta ni detección de CPU en
cada multiplicación.

**ARCH-006.** `Engine<F>` seleccionará un `KernelSet<F>` una vez. Las
operaciones sobre slices harán una sola llamada indirecta por lote.

## 1.5 Stable Rust

**ARCH-007.** El MSRV inicial será Rust 1.89. La versión estable disponible en
la revisión local del 31 de julio de 2026 es Rust 1.93.1. El runtime sin
features y el generador compilan localmente con Rust 1.89.0; CI repetirá ese
gate en cada cambio.

**ARCH-008.** No se usará `std::simd` en la API ni en el backend estable durante
estas fases: sigue siendo una API experimental. Se usarán:

- Rust escalar en `portable`;
- `core::arch::x86_64` en x86-64;
- `core::arch::aarch64` en ARM64.

## 1.6 `unsafe`

**ARCH-009.** `unsafe` solo podrá aparecer en:

```text
src/backend/x86/
src/backend/aarch64/
src/engine/packed/storage.rs
```

Todo `unsafe fn` tendrá una sección `# Safety` que especifique:

- features de CPU;
- punteros válidos;
- longitudes;
- alineamiento;
- aliasing;
- inicialización;
- layout;
- comportamiento del tail.

## 1.7 Identidades

**ARCH-010.** Se distinguirán:

- `FieldId`: identidad de semántica y codificación canónica;
- `ArtifactId`: `FieldId` más versión del generador, representación y perfil;
- `BackendId`: ejecutor concreto seleccionado en una máquina.

Cambiar de PCLMUL a PMULL no cambia `FieldId`. Cambiar el polinomio o el orden
canónico de bytes sí.

# 2. Organización física

## 2.1 Árbol propuesto

```text
microfield/
  Cargo.toml
  README.md
  src/
    lib.rs
    error.rs
    field/
      mod.rs
      traits.rs
      encoding.rs
      pow.rs
    spec/
      mod.rs
      manifest.rs
      normalize.rs
      validate.rs
      identity.rs
      certificate.rs
      plans.rs
      vectors.rs
      emit.rs
    binary/
      mod.rs
      repr.rs
      portable.rs
      reference.rs
      square.rs
      invert.rs
    engine/
      mod.rs
      builder.rs
      policy.rs
      capabilities.rs
      kernel.rs
      dispatch.rs
    packed/
      mod.rs
      layout.rs
      aligned.rs
      view.rs
      convert.rs
    backend/
      mod.rs
      portable.rs
      x86/
        mod.rs
        pclmul.rs
        vpclmul.rs
      aarch64/
        mod.rs
        pmull.rs
    generated/
      mod.rs
      gf2_128_v1.rs
      gf2_256_hh_v1.rs
      gf2_256_alt_v1.rs
    bin/
      microfield-gen.rs
  fields/
    gf2_128_v1.toml
    gf2_256_hh_v1.toml
    gf2_256_alt_v1.toml
  generated-artifacts/
    ...
  certificates/
    ...
  reference-vectors/
    ...
  tests/
    field_laws.rs
    encoding.rs
    plans.rs
    differential.rs
    dispatch.rs
    packed.rs
    compile_fail/
  benches/
    scalar.rs
    batch.rs
    packing.rs
    dispatch.rs
```

Éstos son módulos, no paquetes independientes. Cada directorio expresa una
frontera de revisión y de `unsafe`.

## 2.2 API visible

La raíz reexportará solo:

```rust
pub use field::{
    BinaryPolynomialField,
    CanonicalEncoding,
    ExtensionField,
    Field,
};
pub use engine::{
    BackendId,
    Engine,
    EngineBuilder,
    ExecutionPolicy,
};
pub use packed::PackedBatch;
pub use generated::{
    Gf2_128V1,
    Gf2_256AltV1,
    Gf2_256HhV1,
};
```

`spec` se expondrá únicamente con `feature = "generator"`. Los tipos de
intrinsics, limbs, buffers y planes internos no se reexportarán.

## 2.3 Features

```toml
[features]
default = ["std", "portable", "builtin-fields", "native-backends"]
std = ["alloc"]
alloc = []
portable = []
builtin-fields = []
native-backends = []
generator = ["std", "serde", "serde_json", "toml", "sha2"]
```

Reglas:

- `--no-default-features --features portable,builtin-fields` compila el núcleo
  `no_std`;
- `generator` nunca es dependencia del hot path;
- `native-backends` compila únicamente el módulo correspondiente al target;
- no habrá una feature pública por instrucción individual;
- la selección entre PCLMUL y VPCLMUL se hace en runtime, no con features Cargo.

# 3. Modelo de reutilización

## 3.1 Traits públicos

Los contratos públicos se segregan por capacidad:

```rust
pub trait Field:
    Copy + Clone + Eq + Send + Sync + 'static
{
    const ZERO: Self;
    const ONE: Self;

    fn add(self, rhs: Self) -> Self;
    fn sub(self, rhs: Self) -> Self;
    fn neg(self) -> Self;
    fn mul(self, rhs: Self) -> Self;
    fn is_zero(&self) -> bool;
}

pub trait Square: Field {
    fn square(self) -> Self;
}

pub trait Invert: Field {
    fn invert(self) -> Option<Self>;
}

pub trait Pow: Field + Square {
    fn pow(self, exponent_le: &[u64]) -> Self;
}
```

`pow` interpreta el exponente como palabras little-endian. Su implementación
por defecto usa square-and-multiply y no asigna memoria.

Codificación:

```rust
pub trait CanonicalEncoding: Field {
    type Repr:
        Copy + Clone + Default + AsRef<[u8]> + AsMut<[u8]>;

    fn from_canonical(
        repr: &Self::Repr,
    ) -> Result<Self, DecodeError>;

    fn to_canonical(self) -> Self::Repr;

    fn from_canonical_slice(
        bytes: &[u8],
    ) -> Result<Self, DecodeError>;
}
```

Extensiones:

```rust
pub trait ExtensionField: Field {
    type Base: Field;
    const DEGREE: usize;

    fn frobenius(self, power: usize) -> Self;
    fn trace(self) -> Self::Base;
    fn norm(self) -> Self::Base;
}
```

Campos binarios en base polinómica:

```rust
pub trait BinaryPolynomialField:
    ExtensionField
{
    const MODULUS_DEGREE: usize;

    fn mul_by_x(self) -> Self;

    fn from_polynomial_bytes_mod(
        bytes_le: &[u8],
    ) -> Self;
}
```

`from_polynomial_bytes_mod` interpreta los bits como coeficientes de un
polinomio y reduce módulo $f(X)$. No se llamará `canonical`.

## 3.2 Traits internos

Para reutilizar algoritmos sin exponer representación:

```rust
pub(crate) trait LimbArray:
    Copy + AsRef<[u64]> + AsMut<[u64]>
{}

pub(crate) trait BinaryFieldImpl:
    Field + CanonicalEncoding
{
    type Limbs: LimbArray;
    type Wide: LimbArray;

    const STATIC_SPEC: &'static StaticFieldSpec;

    fn from_limbs(limbs: Self::Limbs) -> Self;
    fn limbs(&self) -> &Self::Limbs;
}
```

Habrá implementaciones de `LimbArray` para arrays concretos:

```rust
[u64; 2], [u64; 4], [u64; 6], [u64; 8]
```

No se dependerá de `generic_const_exprs` para formar `[u64; 2 * N]`.
El generador emitirá los tipos `Limbs` y `Wide` concretos.

Decisión de H2: mientras solo existía una representación grande, el trait
`BinaryFieldImpl` no se materializó como abstracción sin consumidores. H3 ya
introduce el contrato común al coexistir `[u64; 2]` y `[u64; 4]`.
`Polynomial128<TAIL>` y `Polynomial256<TAIL>` encapsulan las estrategias
estáticas, mientras un macro privado genera solo newtypes, metadatos y
delegación. Producto, reducción, cuadrado, inversión y operaciones de extensión
se reutilizan sin dispatch ni lógica matemática duplicada.

## 3.3 Composición

`Engine<F>` contiene un `KernelSet<F>`, no hereda de un backend.
`PackedBatch<F>` contiene un `AlignedBuffer` y un `PackingPlan`, no es un
registro SIMD.

Esta composición permite:

- cambiar backend sin cambiar elementos;
- mantener privado el layout;
- probar cada componente por separado;
- conservar monomorfización en álgebra escalar;
- concentrar el dispatch en una llamada batch.

## 3.4 Tipos de estado

El pipeline de generación usa tipos diferentes:

```rust
FieldManifest
    -> NormalizedManifest
    -> ValidatedFieldSpec
    -> GeneratedArtifacts
    -> tipo de campo compilable
```

No existe una conversión pública que omita pasos.

```mermaid
flowchart LR
    A[FieldManifest] --> B[NormalizedManifest]
    B --> C[ValidatedFieldSpec]
    C --> D[GeneratedArtifacts]
    D --> E[Tipo compilable]
```

# 4. Fase 0 - Especificación y generación

## 4.1 Objetivo

La Fase 0 no implementa todavía un campo rápido. Construye el sistema que
impide generar un campo incorrecto o ambiguo.

**F0-FR-001.** El campo actual debe expresarse sin mencionar `topology`,
`engine`, hashes ni una CPU.

**F0-FR-002.** Un manifiesto inválido debe detener el pipeline antes de emitir
Rust.

**F0-FR-003.** La normalización debe ser determinista en todas las máquinas.

**F0-FR-004.** Todo artefacto generado debe poder vincularse a su manifiesto,
generador, certificado y vectores.

## 4.2 `FieldManifest`

```rust
pub struct FieldManifest {
    pub schema_version: u16,
    pub field: FieldDescriptor,
    pub build: BuildProfile,
}
```

Responsabilidad: representar entrada TOML todavía no confiable.

Funciones:

```rust
impl FieldManifest {
    pub fn parse_toml(
        source: &str,
    ) -> Result<Self, ManifestError>;

    pub fn load(
        path: impl AsRef<Path>,
    ) -> Result<Self, ManifestError>;

    pub fn normalize(
        self,
    ) -> Result<NormalizedManifest, NormalizationError>;
}
```

Procesamiento:

1. parsear tipos sintácticos;
2. rechazar claves desconocidas salvo extensiones versionadas;
3. comprobar límites de longitud;
4. separar datos de identidad de hints de compilación;
5. entregar el valor al normalizador.

No calcula `FieldId`.

## 4.3 `FieldDescriptor`

```rust
pub struct FieldDescriptor {
    pub name: FieldName,
    pub characteristic: Characteristic,
    pub degree: NonZeroU32,
    pub basis: BasisDescriptor,
    pub modulus: Option<PolynomialDescriptor>,
    pub encoding: CanonicalEncodingDescriptor,
    pub basis_element: Option<BasisElementDescriptor>,
    pub primitive_element: Option<PrimitiveElementClaim>,
}
```

### `Characteristic`

```rust
pub enum Characteristic {
    Small(u64),
    Decimal(String),
}
```

En Fases 0-2 solo se ejecutan campos de característica dos. La variante
decimal permite conservar la forma futura sin introducir un entero grande en
el runtime.

### `BasisDescriptor`

```rust
pub enum BasisDescriptor {
    Prime,
    Polynomial {
        coefficient_order: CoefficientOrder,
    },
    Normal {
        descriptor: String,
    },
    Tower {
        layers: Vec<TowerLayer>,
    },
}
```

Estas variantes quedan como diseño prospectivo para un esquema posterior.
El parser v1 rechaza cualquier base distinta de `Polynomial` sobre
$\mathbb F_2$; no conserva variantes parcialmente válidas.

### `PolynomialDescriptor`

```rust
pub struct PolynomialDescriptor {
    pub degree: u32,
    pub nonzero_exponents: Vec<u32>,
}
```

Invariantes normalizados:

- exponentes estrictamente decrecientes;
- primer exponente igual al grado;
- último exponente cero;
- sin duplicados;
- polinomio mónico;
- grado consistente con el campo.

### Elemento de base frente a generador — aplazado

```rust
pub struct BasisElementDescriptor {
    pub canonical_hex: String,
    pub role: BasisElementRole,
}

pub enum BasisElementRole {
    PolynomialX,
    RollingBase,
    NamedConstant,
}

pub struct PrimitiveElementClaim {
    pub canonical_hex: String,
    pub required_order: OrderClaim,
    pub factorization: Option<Factorization>,
}
```

Estos claims no forman parte del manifiesto v1. En H2 el valor canónico `02`
podrá utilizarse en vectores como elemento polinómico `x`, sin afirmar que
genera todo el grupo multiplicativo.

## 4.4 `BuildProfile`

```rust
pub struct BuildProfile {
    pub limb_bits: u16,
    pub product_strategies: Vec<ProductStrategy>,
    pub reduction_style: ReductionStyle,
    pub requested_backends: Vec<BackendFamily>,
}
```

Las políticas de unroll se aplazan hasta que existan benchmarks. En H1 las
estrategias son descriptores de planificación; al emitir código ejecutable H2
deberá rechazar cualquier estrategia todavía no implementada.

No forma parte de `FieldId`. Sí forma parte de `ArtifactId`.

Esto permite generar dos implementaciones del mismo campo con diferente
Karatsuba o layout sin romper ficheros persistentes.

## 4.5 `NormalizedManifest`

```rust
pub struct NormalizedManifest {
    identity: CanonicalFieldDescriptor,
    build: NormalizedBuildProfile,
    canonical_toml: String,
}
```

Solo `spec::normalize` puede construirlo.

Funciones:

```rust
impl NormalizedManifest {
    pub fn canonical_toml(&self) -> &str;
    pub fn identity_bytes(&self) -> &[u8];

    pub fn validate(
        self,
        engine: &ValidationEngine,
    ) -> Result<ValidatedFieldSpec, ValidationError>;
}
```

La normalización:

1. fija el orden de claves;
2. convierte enteros a decimal sin ceros redundantes;
3. normaliza hexadecimal en minúsculas y longitud par;
4. ordena exponentes;
5. explicita defaults semánticos;
6. elimina comentarios y nombres no autoritativos de `identity_bytes`;
7. conserva el TOML legible para diffs.

## 4.6 `FieldId` y `ArtifactId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct FieldId([u8; 32]);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ArtifactId([u8; 32]);
```

Definición:

$$
\operatorname{FieldId}
=
\operatorname{SHA256}
\left(
\texttt{"microfield:field-id:v1\textbackslash0"}
\parallel
\operatorname{identity\_bytes}
\right).
$$

El SHA-256 se usa como identificador de artefacto, no como primitiva de las
futuras firmas algebraicas. La igualdad autoritativa sigue siendo la del
descriptor canónico.

`ArtifactId` incluye:

- `FieldId`;
- versión semántica del generador;
- `BuildProfile` normalizado;
- versión del IR;
- target family, no modelo de CPU.

Funciones:

```rust
impl FieldId {
    pub fn as_bytes(&self) -> &[u8; 32];
    pub fn to_hex(self) -> String;
    pub fn from_hex(value: &str) -> Result<Self, IdError>;
}
```

## 4.7 `ValidationEngine`

```rust
pub struct ValidationEngine {
    pub version: ValidatorVersion,
    pub policy: ValidationPolicy,
}

pub struct ValidationPolicy {
    pub max_degree: u32,
    pub require_irreducibility_certificate: bool,
    pub require_external_vectors: bool,
    pub generator_claims: GeneratorClaimPolicy,
}
```

Método:

```rust
impl ValidationEngine {
    pub fn validate(
        &self,
        manifest: NormalizedManifest,
    ) -> Result<ValidatedFieldSpec, ValidationError>;
}
```

Flujo:

1. validar característica;
2. validar cardinalidad y grado;
3. validar base;
4. comprobar forma del módulo;
5. ejecutar irreducibilidad de Rabin;
6. verificar claim de generador si existe;
7. comprobar encoding;
8. calcular `FieldId`;
9. crear certificados;
10. producir `ValidatedFieldSpec`.

### Irreducibilidad

Para binarios se implementará aritmética polinómica sobre arrays de `u64`.
El verificador ejecuta:

$$
X^{2^m}-X\equiv0\pmod f
$$

y, para cada divisor primo $r$ de $m$:

$$
\gcd\left(X^{2^{m/r}}-X,f\right)=1.
$$

El resultado incluye residuos intermedios suficientes para repetir la
verificación.

## 4.8 `ValidatedFieldSpec`

```rust
pub struct ValidatedFieldSpec {
    field_id: FieldId,
    descriptor: CanonicalFieldDescriptor,
    build: NormalizedBuildProfile,
    validation: ValidationReport,
    certificate: CertificateBundle,
}
```

Los campos son privados.

Lecturas:

```rust
impl ValidatedFieldSpec {
    pub fn field_id(&self) -> FieldId;
    pub fn descriptor(&self) -> &CanonicalFieldDescriptor;
    pub fn validation(&self) -> &ValidationReport;
    pub fn certificate(&self) -> &CertificateBundle;

    pub fn plan(
        &self,
        planner: &GenerationPlanner,
    ) -> Result<GenerationPlan, GenerationError>;
}
```

## 4.9 Certificados

```rust
pub struct CertificateBundle {
    pub field_id: FieldId,
    pub validator: ValidatorVersion,
    pub characteristic: CharacteristicCertificate,
    pub irreducibility: Option<IrreducibilityCertificate>,
    pub primitive_element: Option<GeneratorCertificate>,
    pub identity_descriptor_sha256: [u8; 32],
}
```

El certificado no contiene afirmaciones no verificadas. Si no se proporcionó
factorización suficiente para probar orden primitivo, el campo correspondiente
es `None`.

## 4.10 Planes generados

### `ProductPlan`

```rust
pub struct ProductPlan {
    pub limb_bits: u16,
    pub input_limbs: u16,
    pub wide_limbs: u16,
    pub strategies: Vec<ProductStrategy>,
}

pub enum ProductStrategy {
    Schoolbook,
    Karatsuba1,
    KaratsubaRecursive { base_limbs: u16 },
}
```

Fase 1 implementa `Schoolbook`. Fase 2 puede emitir y medir
`Karatsuba1`.

### `ReductionPlan`

```rust
pub struct ReductionPlan {
    pub input_bits: u32,
    pub output_bits: u32,
    pub steps: Vec<FoldStep>,
    pub requires_second_fold: bool,
    pub proof_digest: [u8; 32],
}

pub struct FoldStep {
    pub source_range: BitRange,
    pub shift: i32,
    pub xor_targets: Vec<BitRange>,
}
```

El plan es independiente de x86 o ARM.

### `ExponentiationPlan`

```rust
pub struct ExponentiationPlan {
    pub purpose: ExponentiationPurpose,
    pub steps: Vec<ExponentiationStep>,
}

pub enum ExponentiationStep {
    Square { count: u32 },
    MultiplyBase,
    MultiplySaved { slot: u16 },
    Save { slot: u16 },
}
```

En Fase 1 genera una cadena fija correcta para $2^m-2$. La optimización
Itoh-Tsujii se reserva para Fase 3.

## 4.11 `GeneratedArtifacts`

```rust
pub struct GeneratedArtifacts {
    pub field_name: String,
    pub field_id: FieldId,
    pub artifact_id: ArtifactId,
    pub bundle_digest: ArtifactBundleDigest,
    pub files: Vec<GeneratedFile>,
}
```

H1.5 emite `normalized.toml`, `descriptor.json`, `certificate.json`,
`generation-plan.json`, `metadata.json`, `field.rs` y `bundle.json`. El
séptimo fichero registra ruta, longitud y SHA-256 de los otros seis. Su
`ArtifactBundleDigest` no participa en `ArtifactId`.

Los vectores externos se mantienen separados y no se presentan como generados
por la propia biblioteca.

La emisión es transaccional:

1. generar todo en memoria o directorio temporal;
2. validar nombres y hashes;
3. ejecutar format/check opcional;
4. mover el conjunto completo;
5. no dejar código parcial si falla.

## 4.12 Vectores — contrato v2 cerrado en H1.5

```rust
pub struct ReferenceVectorSet {
    pub schema: u32,
    pub field_id: String,
    pub oracle: OracleMetadata,
    pub generation: VectorGeneration,
    pub vectors: Vec<ReferenceVector>,
}

pub enum VectorOperation {
    Canonical,
    Add,
    WideProduct,
    Reduce,
    Multiply,
    Square,
    Invert,
    Pow,
    MulByX,
}
```

Cada elemento ocupa `ceil(m/8)` bytes en encoding canónico. Productos anchos y
entradas de reducción ocupan exactamente el doble y usan coeficientes
polinómicos sin reducir. Los exponentes son bytes little-endian mínimos y la
inversa de cero tiene salida `None`.

ADR 0005 fija operaciones, cobertura, identidad, seed, versión del oráculo,
bits de padding y límites de 8 MiB, 4096 casos y 4096 bytes de exponente.
Claves u operaciones desconocidas se rechazan.

## 4.13 CLI integrada

`microfield-gen` tendrá estos subcomandos:

```text
microfield-gen normalize MANIFEST
microfield-gen validate MANIFEST
microfield-gen plan MANIFEST
microfield-gen emit MANIFEST --out DIR
microfield-gen vectors MANIFEST [--oracle-json FILE | --sage PATH] [--out FILE]
microfield-gen certify MANIFEST
microfield-gen check MANIFEST --out DIR
microfield-gen all MANIFEST --out DIR
```

En Fase 2 se añade:

```text
microfield-gen asm-audit ARTIFACT --target TARGET
```

Todos admiten `--json` para CI. Los errores salen por stderr y los resultados
por stdout.

## 4.14 Errores de Fase 0

```rust
pub enum ManifestError {
    Io,
    Syntax,
    UnknownKey,
    UnsupportedSchema,
}

pub enum NormalizationError {
    ConflictingValues,
    InvalidHex,
    InvalidPolynomialShape,
    InvalidEncoding,
}

pub enum ValidationError {
    CompositeCharacteristic,
    DegreeMismatch,
    ReducibleModulus,
    UnsupportedBasis,
    UnprovenGeneratorOrder,
    EncodingCapacityMismatch,
    PolicyViolation,
}

pub enum GenerationError {
    UnsupportedRepresentation,
    NoReductionPlan,
    InvalidPlan,
    OracleMismatch,
    EmissionFailed,
}
```

No se usarán strings como categorías de error.

## 4.15 Pruebas de Fase 0

1. golden tests de normalización;
2. mismo descriptor con orden TOML diferente produce mismo `FieldId`;
3. cambiar módulo produce otro `FieldId`;
4. cambiar `BuildProfile` conserva `FieldId` y cambia `ArtifactId`;
5. polinomios reducibles conocidos se rechazan;
6. módulos irreducibles mantenidos se aceptan;
7. claims de generador incompletos se rechazan o degradan según política;
8. corrupción de certificado se detecta;
9. generación repetida produce bytes idénticos;
10. ningún fichero parcial aparece tras un fallo.

## 4.16 Definición de terminado

Fase 0 termina cuando:

- existe el paquete único y compila;
- los tres manifiestos binarios se normalizan;
- sus módulos se validan con certificado;
- `gf2_256_hh_v1` no contiene ningún concepto de hashes o grafos;
- `microfield-gen all` produce artefactos reproducibles;
- existe un harness vacío pero ejecutable de benchmarks;
- no hay `unsafe`;
- todos los ADR de esta especificación están registrados.

El hito implementado H1 es deliberadamente una Fase 0 mínima. H1.5 cierra el
contrato v2, el digest de bundle y la automatización CI. SageMath 10.7 ha
generado los tres juegos golden, su regeneración es idéntica byte a byte y un
modelo polinómico lento verifica todas las operaciones. La línea base quedó
publicada en `c9671ee` y los cinco jobs del workflow remoto `30592909350`
terminaron correctamente.

# 5. Fase 1 - Vertical binario portable

**Estado H2:** `Gf2_256HhV1` está implementado e integrado en `main` mediante
`f3f7fc3`, con producto portable, reducción const-generic,
cuadrado dedicado, inversión por plan, potencia, `mul_by_x`, Frobenius, traza,
norma y encoding. Supera leyes deterministas, vectores Sage, compatibilidad con
`GaloisSignature256`, Miri y auditoría de ensamblado. Las ejecuciones CI de la
rama y de `main`, `30622165087` y `30622957505`, terminaron correctamente.

**Estado H3:** los tres campos obligatorios están implementados sobre un único
núcleo algebraico estático e integrados en `main` mediante `78d517f`. La rama
y `main` superaron `30624475704` y `30701163784`.

**Estado H4:** el motor batch portable está integrado en `main` y validado:
catálogo sellado por campo, `EngineBuilder`, selección única, operaciones
out-of-place/in-place, errores transaccionales y benchmark bajo el gate de 3 %.
El contador externo confirma cero asignaciones y el ensamblado una única
llamada indirecta por lote. Stable, Clippy, rustdoc, features, MSRV 1.89, Miri,
artefactos deterministas y regresión legada están verdes. La implementación se
desarrolló en `9cbfa15`, quedó integrada mediante `1f176ab` y sus cinco jobs de
cierre terminaron correctamente en `30703842091`. La Fase 1 queda formalmente
cerrada; su inventario está en `docs/microfield/phase-1-final-report.md`.

## 5.1 Campos obligatorios

### `Gf2_128V1`

Campo binario de 128 bits en base polinómica. El módulo exacto queda fijado por
su manifiesto y certificado:

$$
X^{128}+X^7+X^2+X+1.
$$

### `Gf2_256HhV1`

$$
\mathbb F_{2^{256}}
\cong
\mathbb F_2[X]/
\left(
X^{256}+X^{10}+X^5+X^2+1
\right).
$$

Encoding canónico:

- 32 bytes;
- little-endian;
- el bit $i$ representa el coeficiente de $X^i$.

### `Gf2_256AltV1`

Segundo campo de cardinalidad $2^{256}$:

$$
X^{256}+X^{16}+X^3+X+1.
$$

Su función
es probar que:

- misma cardinalidad no significa misma representación;
- `FieldId` evita mezclar elementos persistentes;
- el generador no contiene constantes codificadas para un único módulo.

El manifiesto congela el polinomio, pero el tipo no se publicará hasta que
Fase 0 emita certificado y vectores independientes.

## 5.2 Tipos concretos

```rust
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Gf2_128V1([u64; 2]);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Gf2_256HhV1([u64; 4]);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Gf2_256AltV1([u64; 4]);
```

Reglas:

- los limbs son privados;
- no se aplica `repr(align(32))` al elemento;
- no se deriva `Serialize`;
- `Debug` muestra encoding canónico, no memoria interna;
- no existe conversión `From` entre los dos campos de 256 bits;
- no se implementa `Ord` salvo que se documente como orden de encoding, no
  orden algebraico.

## 5.3 `StaticFieldSpec`

Cada tipo apunta a metadatos inmutables:

```rust
pub struct StaticFieldSpec {
    pub field_id: FieldId,
    pub artifact_id: ArtifactId,
    pub name: &'static str,
    pub characteristic: u64,
    pub degree: u32,
    pub canonical_bytes: u16,
    pub descriptor_json: &'static [u8],
    pub certificate_json: &'static [u8],
}
```

Función pública:

```rust
pub trait StaticField: Field {
    fn spec() -> &'static StaticFieldSpec;
}
```

## 5.4 Representaciones internas

El generador emite por campo:

```rust
type Limbs = [u64; 4];
type Wide = [u64; 8];
```

`Wide` nunca representa un elemento reducido y no sale de `binary`.

Para probar etapas por separado se habilitan bajo `cfg(test)`:

```rust
fn carryless_product_wide(a: Limbs, b: Limbs) -> Wide;
fn reduce_wide(value: Wide) -> Limbs;
fn square_wide(value: Limbs) -> Wide;
```

## 5.5 Suma, resta y negación

En característica dos:

$$
a+b=a-b=a\oplus b,
\qquad
-a=a.
$$

Proceso:

1. cargar limbs;
2. XOR por posición;
3. construir el resultado;
4. no reducir.

Funciones:

```rust
fn add(self, rhs: Self) -> Self;
fn sub(self, rhs: Self) -> Self;
fn neg(self) -> Self;
```

Se implementan también `Add`, `Sub`, `AddAssign` y `SubAssign`.

## 5.6 Producto portable

Proceso normativo:

```text
a, b
  -> carryless_product_wide
  -> Wide de 2L limbs
  -> reduce_wide(plan)
  -> elemento de L limbs
```

### Carry-less de 64 bits

La referencia ejecutable:

```rust
fn clmul64_reference(a: u64, b: u64) -> u128;
```

debe ser legible, sin intrinsics y sin tablas mutables. Puede usar un bucle de
64 pasos. No es el kernel de rendimiento objetivo.

### Producto escolar

Para $L$ limbs:

$$
w_{i+j}\mathrel{\oplus}=a_i\otimes b_j.
$$

La versión generada puede desenrollarse. Para cuatro limbs ejecuta 16 productos
carry-less de 64 bits.

### Reducción

`reduce_wide` aplica `ReductionPlan`. Debe existir además una reducción lenta
por división polinómica bajo tests. Las dos se comparan para entradas anchas.

Función pública:

```rust
fn mul(self, rhs: Self) -> Self;
```

No asigna, no llama a `Engine` y no detecta CPU.

## 5.7 Cuadrado portable

No se implementará como `self.mul(self)`.

Proceso:

1. expandir cada bit $i$ a la posición $2i$;
2. producir `Wide`;
3. reducir;
4. devolver el elemento.

El expansor puede usar máscaras y shifts o una tabla `const`. Debe comprobarse
que:

$$
(a+b)^2=a^2+b^2.
$$

## 5.8 `mul_by_x`

Proceso:

1. desplazar toda la representación un bit;
2. capturar el coeficiente de $X^{m-1}$;
3. si hay overflow, XOR con $r(X)$ donde $f(X)=X^m+r(X)$;
4. enmascarar bits altos si $m$ no es múltiplo de 64.

Función:

```rust
fn mul_by_x(self) -> Self;
```

No se llamará `shift_phase`.

## 5.9 Potencia

```rust
fn pow(self, exponent_le: &[u64]) -> Self;
```

Semántica:

- exponente vacío equivale a cero;
- $0^0$ devuelve `ONE` por convención algorítmica documentada;
- no asigna;
- el tiempo depende de la longitud y bits del exponente;
- no promete tiempo constante.

## 5.10 Inversión

```rust
fn invert(self) -> Option<Self>;
```

Proceso:

1. si es cero, devolver `None`;
2. ejecutar `ExponentiationPlan` para $2^m-2$;
3. usar `square` especializado y `mul`;
4. comprobar en tests que $a\cdot a^{-1}=1$.

Fase 1 prioriza corrección y cadena fija. Fase 3 sustituirá la cadena cuando
Itoh-Tsujii demuestre ventaja.

## 5.11 Frobenius, traza y norma

Para extensiones binarias:

$$
\operatorname{Frob}^k(a)=a^{2^k}.
$$

Implementación:

```rust
fn frobenius(self, power: usize) -> Self {
    let count = power % Self::DEGREE;
    repeat_square(self, count)
}
```

Traza:

$$
\operatorname{Tr}(a)
=
\sum_{i=0}^{m-1}a^{2^i}
\in\mathbb F_2.
$$

Norma:

$$
\operatorname{N}(a)
=
a^{2^m-1}
\in\mathbb F_2.
$$

Se define:

```rust
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct F2(bool);
```

Fase 1 puede implementar traza/norma de forma directa; su optimización queda
fuera del camino crítico.

## 5.12 Encoding canónico

Para `Gf2_256HhV1`:

```rust
impl CanonicalEncoding for Gf2_256HhV1 {
    type Repr = [u8; 32];
}
```

### `from_canonical`

1. validar longitud por el tipo `Repr`;
2. validar bits altos para grados no múltiplos de ocho;
3. leer palabras little-endian;
4. no reducir;
5. devolver exactamente el elemento codificado.

### `from_canonical_slice`

1. comparar longitud;
2. copiar a `Repr` de stack;
3. llamar a `from_canonical`.

### `to_canonical`

1. escribir palabras little-endian;
2. limpiar padding;
3. devolver array.

### Reducción de bytes

```rust
fn from_polynomial_bytes_mod(bytes_le: &[u8]) -> Self;
```

No tiene el mismo contrato. Evalúa el polinomio por bloques y reduce.

## 5.13 `PortableKernelSet`

Fase 1 crea el primer `KernelSet`:

```rust
pub(crate) static PORTABLE_KERNELS:
    KernelSet<Gf2_256HhV1>;
```

Las funciones batch son bucles sin asignación:

```rust
fn add_portable(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
);

fn mul_portable(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
);

fn square_portable(
    out: &mut [F],
    values: &[F],
);
```

## 5.14 API batch portable

`Engine::portable()` estará disponible en Fase 1:

```rust
impl<F: PortableField + StaticField> Engine<F> {
    pub fn portable() -> Self;

    pub fn add_into(
        &self,
        out: &mut [F],
        lhs: &[F],
        rhs: &[F],
    ) -> Result<(), BatchError>;

    pub fn mul_into(
        &self,
        out: &mut [F],
        lhs: &[F],
        rhs: &[F],
    ) -> Result<(), BatchError>;

    pub fn square_into(
        &self,
        out: &mut [F],
        values: &[F],
    ) -> Result<(), BatchError>;

    pub fn mul_assign(
        &self,
        lhs: &mut [F],
        rhs: &[F],
    ) -> Result<(), BatchError>;

    pub fn square_assign(
        &self,
        values: &mut [F],
    );
}
```

Reglas:

- slices vacíos son válidos;
- longitudes incompatibles devuelven error antes de escribir;
- si falla validación, `out` permanece intacto;
- `*_into` no permite aliasing en Rust seguro;
- `*_assign` es la ruta in-place;
- no hay asignaciones;
- no se paraleliza por hilos.

## 5.15 `BatchError`

```rust
pub enum BatchError {
    LengthMismatch {
        out: usize,
        lhs: usize,
        rhs: Option<usize>,
    },
    IncompatiblePacking,
    BackendUnavailable,
}
```

## 5.16 Tests algebraicos

Para cada campo:

$$
(a+b)+c=a+(b+c),
$$

$$
(ab)c=a(bc),
$$

$$
a(b+c)=ab+ac,
$$

$$
a+0=a,\qquad a\cdot1=a,
$$

$$
a\ne0\Longrightarrow a\,a^{-1}=1,
$$

$$
a+a=0,
$$

$$
(a+b)^2=a^2+b^2,
$$

$$
a^{2^m}=a.
$$

Además:

- cada bit individual;
- productos densos;
- límites de limbs;
- cero, uno y máximo canónico;
- reducción rápida contra división polinómica;
- vectores de Sage/NTL;
- roundtrip de encoding;
- confusión entre campos rechazada por el compilador.

## 5.17 Tests batch

Tamaños:

```text
0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64,
255, 256, 1024, 16384
```

Para cada operación:

```text
resultado batch == map(resultado escalar)
```

También:

- `mul_assign` contra `mul_into`;
- canarios alrededor de buffers;
- el error no modifica salida;
- slices no alineados naturalmente permitidos en portable.

## 5.18 Requisitos de rendimiento

El portable es especificación ejecutable, no objetivo final. Aun así:

- cero asignaciones por operación;
- tamaño de `Gf2_256HhV1` exactamente 32 bytes;
- operaciones escalares sin indirect calls;
- batch portable con una sola validación de longitud;
- benchmark separado de producto ancho y reducción;
- prohibido afirmar optimización SIMD en esta fase.

## 5.19 Definición de terminado

Fase 1 termina cuando:

- los tres campos compilan;
- todas las operaciones mínimas funcionan;
- el cuadrado tiene ruta propia;
- la inversión usa plan fijo;
- encoding es estable;
- el portable pasa vectores independientes;
- batch y escalar coinciden;
- `no_std` compila;
- el campo actual tiene pruebas de compatibilidad;
- no existe `unsafe`.

# 6. Fase 2 - factory binaria pública, x86-64, AArch64 y motor

## 6.1 Objetivo

Fase 2 comienza abriendo la generación estática de campos binarios para
consumidores externos. Después introduce aceleración sin modificar:

- los tipos de elemento;
- el encoding;
- `FieldId`;
- resultados;
- API algebraica escalar.

La factory no crea contextos dinámicos ni devuelve un campo heterogéneo en
runtime. Recibe una definición de GF(2^m), la valida y genera antes de compilar
un tipo Rust nominal, monomorfizado y sin coste adicional en el hot path.

## 6.1.1 Secuencia de hitos

| Hito | Entrega | Dependencia |
|---|---|---|
| H2.1 ✅ | `BinaryFieldFactory` pública y consumidor externo portable | Fase 1 |
| H2.2 ✅ | optimizador portable estático para campos generados | H2.1 |
| H2.3 ✅ | capacidades de CPU, catálogo ampliado y selección única | H2.2 |
| H2.4 ✅ | backend x86 PCLMUL | H2.3 |
| Puente ABI 3 ✅ | perfiles ISA verificados para campos externos | H2.4 |
| H2.5 ✅ | backend AArch64 PMULL | H2.3/Puente ABI 3 |
| H2.6 ✅ | `PackedBatch`, storage alineado y vistas | H2.4/H2.5 |
| H2.7 ✅ | VPCLMUL y layouts de throughput | H2.6 |
| H2.8 ✅ | calibración, auditoría, CI multi-ISA y cierre | H2.4-H2.7 |

PCLMUL y PMULL son ramas independientes después de H2.3. VPCLMUL es
condicional: puede quedar implementado pero no seleccionado si no demuestra
ventaja total incluyendo packing.

El plan operativo, gates y entregables de cada hito se mantienen en
`docs/microfield/phase-2-plan.md`.

## 6.1.2 H2.1 — Factory pública de campos binarios

El primer hito convierte el pipeline interno en una frontera pública de
generación estática. El dominio inicial permanece deliberadamente limitado a:

- característica dos;
- grado de extensión explícito;
- base polinómica;
- módulo mónico expresado por exponentes o bytes canónicos;
- encoding little-endian ya congelado por el esquema v1.

Dentro de ese dominio se soportan grados 2..=4096. La factory emite los
tamaños literales de limbs, producto ancho y representación canónica, además de
la máscara de padding. La reducción usará el plan completo generado y no
quedará limitada a módulos cuyo tail cabe en un `u64`. Los perfiles 128/256
actuales pueden conservar especializaciones si el ensamblado demuestra que son
mejores que la ruta portable general.

Contrato objetivo:

```rust
let package = BinaryFieldFactory::builder()
    .name("gf2_233_custom")
    .degree(233)
    .modulus_exponents([233, 74, 0])
    .build()?
    .generate()?;

package.emit_rust(output_dir)?;
```

También se soporta `BinaryFieldFactory::from_manifest(path)` para `build.rs`.
La salida es código Rust determinista que declara un newtype nominal,
implementa los traits algebraicos, adjunta identidad/certificado/planes y
registra la estrategia portable sin exponer `KernelSet` o punteros de función.

Reglas de la frontera:

- la factory vive bajo `feature = "generator"` y puede usar `std`/`alloc`;
- el tipo generado conserva `no_std`, layout fijo y dispatch escalar estático;
- el manifiesto nunca puede inyectar tokens Rust arbitrarios;
- nombres, rutas, grado y tamaño tienen límites duros;
- Rabin y la validación completa preceden a cualquier emisión;
- la emisión usa staging y no puede escapar del directorio autorizado;
- el ABI de codegen queda versionado;
- los tres campos mantenidos se regeneran por el mismo camino público;
- un campo externo recibe scalar y batch portable en H2.1;
- ningún campo externo activa ISA hasta superar un perfil de compatibilidad
  explícito en un hito posterior.

`BuiltinField` continúa identificando presets mantenidos y catálogos ISA
internos. Para portable, `Engine<F>` acepta la capability segura que la factory
implementa para cada tipo y la raíz de composición crea internamente un
`KernelSet` seguro. No se ha abierto la
construcción pública de catálogos raw.

Criterios de salida H2.1:

1. un crate fixture externo genera, compila y usa un GF(2^m) no mantenido;
2. añadirlo no modifica ningún fichero de `microfield/src/generated`;
3. dos generaciones independientes son byte a byte idénticas;
4. el campo satisface leyes, encoding, Rabin y referencia polinómica lenta;
5. dos definiciones producen tipos nominales incompatibles;
6. scalar y batch portable coinciden en todos los tamaños normativos;
7. `no_std` del runtime generado compila sin activar el generador;
8. no aparece `unsafe` ni asignación en sus operaciones.

Estado: vertical implementado. El fixture externo genera GF(2⁹) y GF(2²³³),
compila en `no_std`, contiene pruebas compile-fail y usa scalar y batch. Los
presets atraviesan la factory para verificar identidad, aunque conservan sus
especializaciones 128/256 mientras sigan ganando en codegen. El ABI de codegen
v1 se documenta en ADR 0010.

## 6.1.3 H2.2 — Optimizador portable estático

La factory no debe limitarse a ofrecer corrección mediante bucles bit a bit.
El segundo hito selecciona durante generación un perfil auditable sin cambiar
el tipo, `FieldId`, encoding o API:

- producto escolar carry-less por bits activos;
- cuadrado dedicado por expansión de bits;
- reducción `LowTailFold` para grados alineados y tails de grado máximo 32;
- reducción por términos para módulos dispersos;
- reducción por palabras para módulos densos;
- inversión mediante cadena binaria Itoh–Tsujii.

La clase de grado distingue potencias de dos alineadas, otros grados alineados
y grados no alineados. Las potencias de dos 64..=4096 tienen prioridad, pero la
estructura del módulo prevalece: un grado no alineado con módulo disperso
también recibe optimización y un módulo denso nunca fuerza codegen expandido
sin cota.

El plan portable forma parte del IR v2 y de `ArtifactId`; `FieldId` permanece
idéntico. ABI de codegen 2 usa helpers nuevos y el runtime conserva ABI 1 para
compatibilidad N-1. La implementación v1 queda como oráculo diferencial.

Estado: implementado y medido. Las rutas se comparan con v1 en grados
64/128/256/512/1024/2048/4096, con un fixture denso GF(2¹⁰), con el modelo exhaustivo
GF(2⁹) y con vectores SageMath 10.7 de GF(2²³³). Los resultados locales están
en `docs/microfield/portable-optimizer.md` y no se convierten en garantías
universales de latencia.

## 6.1.4 H2.3 — Capabilities y selector inmutable

Antes de introducir instrucciones ISA, la selección separa cuatro dimensiones:
backend compilado, estrategia certificada para el campo, soporte real de CPU y
política. `CpuCapabilities` tiene campos privados y solo se obtiene mediante
detección real con `std` o `portable_only`. Los tests internos pueden recorrer
combinaciones sintéticas sin permitir que un consumidor falsee soporte ISA.

`KernelCatalog<F>` contiene portable obligatorio y slots opcionales para
PCLMUL, VPCLMUL y PMULL. Los módulos generados ABI 1/2 heredan un catálogo
portable; ABI 3 adjunta adapters ISA propiedad del runtime mediante un perfil
verificado. `EngineBuilder` elige una vez y `Engine` no almacena capabilities
ni vuelve a detectar.

Semántica implementada:

- `Auto` usa capabilities y el tamaño esperado como umbral;
- `LowLatency` prioriza PCLMUL/PMULL frente a vectorización;
- `Throughput` prioriza VPCLMUL;
- `PortableOnly` impide cualquier ISA;
- `FixedSchedule` exige metadata de schedule fijo;
- el backend forzado valida build, campo, CPU y política, en ese orden.

Estado: implementado. La tabla unitaria cubre exhaustivamente las combinaciones
forzadas y la matriz automática. Integración verifica detección real,
concurrencia, cero asignaciones, `no_std` y compatibilidad ABI 1..=3. H2.4
activa PCLMUL en x86-64 y H2.5 activa PMULL en AArch64; ambos exigen
capabilities detectadas y catálogos certificados.

## 6.1.5 Puente ABI 3 — perfiles ISA externos verificados

Después de normalización, Rabin y planificación, todo campo válido del esquema
v1 recibe un `VerifiedIsaProfile` target-neutral. El perfil autentica `FieldId`,
layout, tamaños de limb/producto, digest de reducción, backends compatibles,
política `explicit_only` y schedule completo. Se publica como
`verified-isa-profile.json`, participa en `ArtifactId`/bundle y su digest queda
embebido en la fuente.

La fuente generada solo implementa un contrato seguro con arrays por valor y
reducción generada. `VerifiedIsaStrategy` permanece opaca y construye dentro de
Microfield los adapters PCLMUL/PMULL; el consumidor no puede registrar punteros,
intrinsics ni falsificar capabilities. `Auto` conserva portable y una ISA solo
se alcanza con `force_backend` tras `detect()`.

El mismo módulo ABI 3 compila scalar-only `no_std` sin activar `portable`; en
esa configuración el perfil no arrastra `Engine` ni adapters ISA. Activar batch
no altera layout, identidad ni aritmética escalar.

El producto schoolbook tiene calendario fijo. Low-tail publica perfil completo
`fixed`; sparse/dense publican `data_dependent` porque su reducción inspecciona
bits del producto. Esta clasificación autenticada gobierna `FixedSchedule`.
Grados 9, 10 denso, 128, 192 y 233 prueban las tres clases estructurales y las
tres familias de reducción bajo x86-64 y AArch64.

## 6.2 `BackendId`

```rust
#[non_exhaustive]
pub enum BackendId {
    Portable,
    X86Pclmul,
    X86Vpclmul,
    Aarch64Pmull,
}
```

AVX-512, SVE y RISC-V quedan fuera de Fase 2.

## 6.3 `CpuCapabilities`

```rust
pub struct CpuCapabilities {
    architecture: Architecture,
    features: u8,
}
```

Funciones:

```rust
impl CpuCapabilities {
    pub fn detect() -> Self;
    pub const fn portable_only() -> Self;
    pub const fn architecture(self) -> Architecture;
    pub const fn has_x86_pclmulqdq(self) -> bool;
    pub const fn has_x86_avx2(self) -> bool;
    pub const fn has_x86_vpclmulqdq(self) -> bool;
    pub const fn has_aarch64_neon(self) -> bool;
    pub const fn has_aarch64_pmull(self) -> bool;
}
```

`detect` existe con `std`. En `no_std` solo se ofrece `portable_only`.

En x86 se usan:

```rust
is_x86_feature_detected!("pclmulqdq")
is_x86_feature_detected!("avx2")
is_x86_feature_detected!("vpclmulqdq")
```

En AArch64 se comprueban `neon/asimd` y `pmull` o la capability equivalente
expuesta por la plataforma. El wrapper de `vmull_p64` se compila con las
features requeridas `neon` y `aes`.

## 6.4 `ExecutionPolicy`

```rust
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ExecutionPolicy {
    Auto,
    LowLatency,
    Throughput,
    PortableOnly,
    FixedSchedule,
}
```

Semántica:

- `Auto`: selector por tamaño y capacidades;
- `LowLatency`: evita packing costoso y favorece PCLMUL por elemento;
- `Throughput`: favorece VPCLMUL y layouts persistentes;
- `PortableOnly`: prohíbe backends ISA;
- `FixedSchedule`: selecciona kernels de control regular cuando existan; no
  promete por sí mismo tiempo constante.

## 6.5 `KernelMetadata`

```rust
pub struct KernelMetadata {
    pub backend: BackendId,
    pub minimum_batch: usize,
    pub preferred_multiple: usize,
    pub required_alignment: usize,
    pub supports_in_place: bool,
    pub requires_packing: bool,
    pub scratch_bytes_per_element: usize,
    pub schedule: ScheduleKind,
}
```

Estos datos son consumidos por el selector y expuestos para diagnóstico.

## 6.6 `KernelSet<F>`

```rust
pub(crate) type BinaryKernel<F> = fn(
    out: &mut [F],
    lhs: &[F],
    rhs: &[F],
);

pub(crate) type UnaryKernel<F> = fn(
    out: &mut [F],
    values: &[F],
);

pub(crate) struct KernelSet<F: Field> {
    pub metadata: KernelMetadata,
    pub add: BinaryKernel<F>,
    pub mul: BinaryKernel<F>,
    pub square: UnaryKernel<F>,
    pub pack: Option<PackKernel<F>>,
    pub unpack: Option<UnpackKernel<F>>,
}
```

`KernelSet` no es construible por usuarios. Solo los módulos internos y el
código generado registran catálogos. En Fase 1 el ABI es completamente seguro.
En Fase 2 cada función segura encapsula el `unsafe` estrictamente local de su
backend ISA después de validar las capacidades al construir el motor.

## 6.7 `KernelCatalog<F>`

```rust
pub(crate) struct KernelCatalog<F: Field> {
    pub portable: &'static KernelSet<F>,
    pub x86_pclmul: Option<&'static KernelSet<F>>,
    pub x86_vpclmul: Option<&'static KernelSet<F>>,
    pub aarch64_pmull: Option<&'static KernelSet<F>>,
}
```

Cada campo generado implementa:

```rust
pub(crate) trait BuiltinField: Field {
    fn kernel_catalog() -> &'static KernelCatalog<Self>;
}
```

`BuiltinField` permanece sellado para presets mantenidos y registro de kernels
ISA. H2.1 abre la generación externa mediante un contrato distinto y seguro:
el código generado aporta descripción matemática y operaciones portables, pero
no puede construir `KernelSet`, registrar punteros ni afirmar compatibilidad
ISA. Los backends acelerados se habilitan únicamente tras comprobar un perfil
de layout y operaciones soportadas dentro del selector.

## 6.8 `EngineBuilder<F>`

```rust
pub struct EngineBuilder<F: PortableField> {
    policy: ExecutionPolicy,
    expected_batch: Option<usize>,
    forced_backend: Option<BackendId>,
    capabilities: CpuCapabilities,
    _field: PhantomData<F>,
}
```

Funciones:

```rust
impl<F: PortableField> EngineBuilder<F> {
    pub fn new() -> Self;

    pub fn policy(
        self,
        policy: ExecutionPolicy,
    ) -> Self;

    pub fn expected_batch(
        self,
        len: usize,
    ) -> Self;

    pub fn force_backend(
        self,
        backend: BackendId,
    ) -> Self;

    pub fn capabilities(
        self,
        capabilities: CpuCapabilities,
    ) -> Self;

    #[cfg(feature = "std")]
    pub fn detect(
        self,
    ) -> Result<Engine<F>, EngineBuildError>;

    pub fn build(
        self,
    ) -> Result<Engine<F>, EngineBuildError>;
}
```

`force_backend` no omite la comprobación de CPU. Si no está disponible devuelve
un error que distingue backend no compilado, campo no elegible y CPU sin las
features necesarias. `build` parte de `portable_only` y no detecta
implícitamente; `detect` toma la instantánea real y termina la construcción.

## 6.9 `Engine<F>`

```rust
#[derive(Clone, Copy)]
pub struct Engine<F: PortableField> {
    kernels: &'static KernelSet<F>,
    policy: ExecutionPolicy,
}
```

Funciones:

```rust
impl<F: BuiltinField> Engine<F> {
    pub fn detect() -> Result<Self, EngineBuildError>;
    pub fn portable() -> Self;
    pub fn backend_id(&self) -> BackendId;
    pub fn metadata(&self) -> &KernelMetadata;
    pub fn policy(&self) -> ExecutionPolicy;

    // add_into, mul_into, square_into,
    // mul_assign y square_assign.
}
```

`Engine` es inmutable después de construirlo. Puede compartirse entre hilos.

## 6.10 Selección

Proceso exacto:

1. obtener catálogo del campo;
2. detectar o recibir capabilities;
3. eliminar kernels no compilados;
4. eliminar kernels incompatibles con CPU;
5. si hay backend forzado, validar campo y política y terminar;
6. excluir perfiles `automatic_selection = false`;
7. aplicar `PortableOnly` y la clasificación de schedule;
8. estimar coste con `expected_batch`;
9. elegir `KernelSet`;
10. guardar la estrategia y no repetir detección en operaciones.

Si `expected_batch` no está presente, `Auto` elige un backend conservador para
lotes medianos. La operación puede usar un subkernel de tail, pero no cambiar
el `BackendId` principal.

## 6.11 Flujo batch

Para `mul_into`:

1. comparar longitudes;
2. retornar antes de escribir si fallan;
3. obtener punteros una vez;
4. calcular tiles completos;
5. ejecutar tail;
6. retornar `Ok(())`.

El wrapper seguro es responsable de longitudes y aliasing. El kernel no repite
comprobaciones por elemento.

## 6.12 x86 PCLMUL

Intrinsic base:

```rust
_mm_clmulepi64_si128
```

El backend tendrá:

```rust
#[target_feature(enable = "pclmulqdq")]
unsafe fn mul_pclmul_256(...);

#[target_feature(enable = "pclmulqdq")]
unsafe fn square_pclmul_256(...);
```

Funciones internas:

1. cargar limbs sin exigir que el elemento tenga alineamiento de 16 bytes;
2. emitir productos carry-less de 64 bits;
3. combinar parciales por XOR;
4. aplicar reducción generada;
5. almacenar resultado.

Se implementarán dos estrategias:

- escolar;
- Karatsuba de un nivel.

Ambas deben producir el mismo resultado. Solo se registra como preferida la que
gane en la familia de CPU medida.

Estado H2.4: implementado. Los tres presets usan Karatsuba (tres productos en
128 bits y nueve en 256), cuadrado dedicado y los reductores ya certificados.
El selector requiere detección real, `PortableOnly` permanece intacto y los
campos externos ABI 3 usan el adapter schoolbook explícito del runtime. Una
fuente ABI 1/2 sin perfil recibe `BackendUnsupportedByField`. ASan, canarios,
longitudes 0..16 384, in-place, cero asignaciones y desensamblado están
cubiertos. En el i7-13700HX medido, el límite conservador de mejora supera 20 %
desde un elemento; `minimum_batch` queda fijado en 1 para los presets. La
frontera completa se documenta en
`docs/microfield/adr/0013-x86-pclmul-backend.md`.

## 6.13 x86 VPCLMUL

Intrinsic estable de 256 bits:

```rust
_mm256_clmulepi64_epi128
```

Procesa un producto de 64 bits por cada una de dos lanes de 128 bits. El
backend organizará varios elementos en SoA/tiles para mantener productos
independientes.

Requisitos:

- `vpclmulqdq`;
- `avx2`;
- batch mínimo medido;
- packing explícito o kernel AoS específico;
- `vzeroupper` según lo que emita/garantice el compilador y la frontera ABI.

No se añadirá AVX-512 en Fase 2.

## 6.14 AArch64 PMULL

Intrinsic base:

```rust
vmull_p64(a: u64, b: u64) -> u128
```

Wrapper:

```rust
#[target_feature(enable = "neon,aes")]
unsafe fn mul_pmull_256(...);
```

Proceso:

1. cargar limbs;
2. realizar productos PMULL;
3. combinar producto ancho;
4. reducir con el mismo `ReductionPlan`;
5. almacenar.

El uso de la mitad alta/PMULL2 se añade si los intrinsics estables o un wrapper
estrecho permiten demostrar ventaja. PMULL básico es requisito; PMULL2 es una
optimización condicionada, no una dependencia de la API.

Estado H2.5: implementado. Los presets usan Karatsuba con 3/9 PMULL y cuadrado
dedicado con 2/4; los perfiles ABI 3 usan schoolbook monomorfizado. Ambas rutas
reutilizan reductores certificados, aceptan toda longitud, soportan in-place y
no asignan. El selector exige NEON + PMULL detectados y el backend permanece
`explicit_only` hasta calibración en hardware ARM real.

QEMU 8.2 `-cpu max` ejecuta tres tests específicos sobre los presets y 11 del
consumidor externo, también bajo AddressSanitizer. El audit release exige
PMULL, especializaciones 128/256 y ausencia de `br`/`blr`/asignador. QEMU no
produce cifras de rendimiento. La decisión se documenta en
`docs/microfield/adr/0015-aarch64-pmull-backend.md`.

## 6.15 `PackingPlan`

```rust
pub struct PackingPlan {
    // campos privados: backend, FieldId, layout, longitudes, tile,
    // limb_count, element_size, alignment y data_bytes
}

#[non_exhaustive]
pub enum PackedLayout {
    Aos,
}
```

La selección del layout depende de campo y backend, no del usuario. H2.6
publica únicamente AoS, el estado consumible por portable/PCLMUL/PMULL. SoA y
tiles híbridos se incorporarán con H2.7; no se modelan variantes futuras
parcialmente válidas.

## 6.16 `AlignedBuffer`

```rust
pub(crate) struct AlignedBuffer<F: Copy> {
    ptr: NonNull<F>,
    len: usize,
    layout: Option<Layout>,
    field: PhantomData<F>,
}
```

Responsabilidades:

- asignar mediante `alloc::alloc::alloc`;
- mantener `Layout`;
- inicializar antes de lectura;
- liberar en `Drop`;
- comprobar overflow de tamaños;
- no exponer slice tipado sin verificar.

Es el único `unsafe` de `packed`.

## 6.17 `PackedBatch<F>`

```rust
pub struct PackedBatch<F: PortableField + StaticField> {
    storage: AlignedBuffer<F>,
    plan: PackingPlan,
}
```

Funciones:

```rust
impl<F: PortableField + StaticField> PackedBatch<F> {
    pub fn new(
        engine: &Engine<F>,
        len: usize,
    ) -> Result<Self, PackError>;

    pub fn from_aos(
        engine: &Engine<F>,
        values: &[F],
    ) -> Result<Self, PackError>;

    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn backend_id(&self) -> BackendId;
    pub fn plan(&self) -> &PackingPlan;

    pub fn pack_from(
        &mut self,
        values: &[F],
    ) -> Result<(), PackError>;

    pub fn unpack_into(
        &self,
        out: &mut [F],
    ) -> Result<(), PackError>;
}
```

Reglas:

- construcción y packing son explícitos;
- el padding se inicializa a cero;
- `len` es lógico, no padded;
- no implementa serialización;
- un batch está ligado a `BackendId` y plan;
- cambiar de engine exige repacking;
- no se accede a registros SIMD desde la API.

## 6.18 Operaciones packed

```rust
impl<F: BuiltinField> Engine<F> {
    pub fn mul_packed_into(
        &self,
        out: &mut PackedBatch<F>,
        lhs: &PackedBatch<F>,
        rhs: &PackedBatch<F>,
    ) -> Result<(), PackError>;

    pub fn square_packed_into(
        &self,
        out: &mut PackedBatch<F>,
        values: &PackedBatch<F>,
    ) -> Result<(), PackError>;

    pub fn mul_packed_assign(...)
        -> Result<(), PackError>;
    pub fn square_packed_assign(...)
        -> Result<(), PackError>;
}
```

Validación:

- mismo campo por tipo;
- mismo backend;
- mismo layout;
- misma longitud;
- salida distinta o ruta explícita in-place.

## 6.19 Vista sin asignación

Para `no_std + alloc` limitado o scratch proporcionado:

```rust
pub struct PackedBatchView<'a, F> { ... }
pub struct PackedBatchViewMut<'a, F> { ... }

pub fn required_packed_bytes(
    plan: &PackingPlan,
) -> Result<usize, PackError>;

pub fn pack_into_storage<'a, F>(
    engine: &Engine<F>,
    storage: &'a mut [MaybeUninit<u8>],
    values: &[F],
) -> Result<PackedBatchViewMut<'a, F>, PackError>;
```

El batch owned es ergonomía bajo `alloc`; la vista está disponible con
`portable` sin `alloc`. Las mismas operaciones out-of-place e in-place existen
para vistas.

## 6.20 Errores de Fase 2

```rust
pub enum EngineBuildError {
    BackendNotCompiled(BackendId),
    BackendUnsupportedByCpu(BackendId),
    BackendUnsupportedByField(BackendId),
    PolicyUnsatisfied(ExecutionPolicy),
}

pub enum PackError {
    LengthMismatch { expected: usize, actual: usize },
    SizeOverflow,
    ZeroSizedField,
    InvalidAlignment { alignment: usize },
    AllocationFailed,
    WrongBackend { expected: BackendId, actual: BackendId },
    IncompatiblePlan,
    InsufficientStorage {
        required: usize,
        provided: usize,
    },
}
```

**Estado H2.6:** implementado. Los tres presets y los campos externos usan el
mismo contrato. El storage owned y prestado pasa longitudes límite, offsets de
alineamiento, Miri/ASan, errores transaccionales y contador de cero asignaciones
durante reutilización. El benchmark separa pack, unpack, kernel persistente y
pipeline total. Véase `docs/microfield/adr/0016-persistent-packed-batches.md`.

## 6.21 Pruebas diferenciales

Para cada campo y vector:

```text
portable == x86_pclmul
portable == x86_vpclmul
portable == aarch64_pmull
```

Operaciones:

- producto;
- cuadrado;
- add batch;
- tail;
- in-place;
- packing y unpacking;
- lotes no múltiplos del tile.

## 6.22 Seguridad de memoria

Pruebas:

- Miri en wrappers portables y `PackedBatch`;
- AddressSanitizer en backends;
- canarios de memoria;
- longitudes cero;
- overflow al calcular tamaño;
- alineamientos 8, 16, 32 y 64;
- buffers con direcciones no preferidas en API AoS;
- tails de 1 a `tile - 1`;
- compile-fail para aliasing seguro.

QEMU sirve para corrección cruzada. No se usará para cifras de rendimiento.

## 6.23 Auditoría de instrucciones

Los scripts de auditoría de Fase 2 comprueban:

- presencia de `pclmulqdq` en kernel PCLMUL;
- presencia de VPCLMUL en kernel correspondiente;
- presencia de PMULL en ARM;
- ausencia de llamadas a asignador;
- ausencia de división;
- ausencia de dispatch dentro del loop;
- tamaño de código;
- regresiones en desensamblado normalizado.

No se exigirá una secuencia byte a byte idéntica entre compiladores.

## 6.24 Benchmarks

### Operaciones

- add;
- mul;
- square;
- pow/invert portable;
- packing;
- unpacking;
- pipeline AoS -> packed -> kernel -> AoS;
- coste de construir Engine;
- coste directo de kernel;
- coste de wrapper Engine.

### Lotes

```text
1, 2, 4, 8, 16, 32, 64, 256, 1024, 16384
```

### Métricas

- ns/elemento;
- ciclos/elemento cuando sea fiable;
- operaciones/segundo;
- bytes/ciclo;
- coste total con packing;
- punto de equilibrio;
- intervalo de confianza;
- CPU, microcódigo, SO, Rust y flags.

## 6.25 Política de aceptación de rendimiento

Un backend se registra para una región si:

1. es correcto;
2. su límite inferior de mejora supera 20 % frente al portable;
3. el coste total incluye packing cuando sea necesario;
4. el selector no lo usa por debajo de su punto de equilibrio;
5. Engine reutilizado queda a menos de 3 % del kernel directo para lotes
   grandes.

Objetivo del hito:

- al menos un kernel PCLMUL y uno PMULL superan 2x al portable en
  `Gf2_256HhV1`;
- VPCLMUL solo se acepta si mejora el throughput total en su región;
- ninguna optimización se mantiene únicamente porque usa una instrucción más
  moderna.

El objetivo PMULL de 2x no se interpreta como demostrado sin cifras nativas.
La salida estable es conservar el backend correcto y forzable, pero excluido de
`Auto`. La ausencia de evidencia nunca se convierte en una estimación.

## 6.26 Definición de terminado

Fase 2 termina cuando:

- un crate consumidor genera GF(2^m) externos sin editar Microfield;
- Builder y manifiesto producen el mismo paquete determinista;
- presets y campos externos comparten pipeline y portable;
- existe detección y selección una sola vez;
- PCLMUL y PMULL son correctos;
- VPCLMUL funciona o queda documentadamente desactivado si no gana;
- `PackedBatch` amortiza packing en una región publicada;
- todos los backends son bit a bit intercambiables;
- `unsafe` está confinado y auditado;
- hay CI de corrección en hardware real x86-64 y AArch64;
- se publican benchmarks por CPU;
- `PortableOnly` sigue funcionando;
- la API pública no contiene tipos ISA.

Estado H2.8: cerrado conservadoramente. La tabla v1 materializa nueve
decisiones estáticas; PCLMUL mantiene su región publicada y PMULL/VPCLMUL quedan
explícitos. La captura multi-runner produce perfiles candidatos con entorno y
SHA-256, mientras la CI ordinaria valida contratos sin usar tiempos ruidosos.
El inventario `unsafe`, el corpus diferencial y la matriz ABI completan los
gates. Véase `docs/microfield/phase-2-final-report.md`.

# 7. Procesos end-to-end

## 7.1 Generar un campo binario externo

1. el consumidor crea `fields/nombre.toml`;
2. `build.rs` invoca `BinaryFieldFactory::from_manifest`;
3. la factory normaliza, aplica límites y valida con Rabin;
4. deriva `FieldId`, planes de reducción/inversión y metadata;
5. emite un módulo Rust determinista dentro de `OUT_DIR`;
6. el crate incluye el módulo generado;
7. el tipo implementa API escalar y batch portable;
8. la CI del consumidor regenera y compara contra su golden o digest fijado.

No se edita `microfield/src/generated` y no se construyen catálogos raw. Para
promover una definición externa a preset mantenido se añaden además manifiesto,
certificado, vectores Sage y artefactos al repositorio de Microfield, usando la
misma factory sin una ruta matemática alternativa.

## 7.2 Multiplicación escalar

```text
Gf2_256HhV1::mul
  -> portable::wide_product
  -> portable::reduce
  -> Gf2_256HhV1
```

No consulta `Engine`.

## 7.3 Multiplicación batch AoS

```text
Engine::mul_into
  -> validar longitudes
  -> KernelSet::mul
  -> tiles completos
  -> tail
  -> salida AoS
```

No asigna.

## 7.4 Pipeline packed

```text
Engine::packing_plan
  -> PackedBatch::from_aos
  -> varias operaciones packed
  -> unpack_into
```

El coste de packing se paga una vez y se amortiza.

## 7.5 Arranque reproducible

```text
tipo concreto
  -> KernelCatalog estático
  -> CpuCapabilities::detect
  -> política + tamaño esperado
  -> KernelSet
  -> Engine inmutable
```

`FieldId` no participa en detección: el tipo ya fija el campo.

# 8. Contratos transversales

## 8.1 Panics

No habrá panic por:

- bytes no canónicos;
- manifiestos inválidos;
- longitudes batch;
- backend forzado no disponible;
- cero al invertir;
- storage insuficiente.

Un panic solo indica:

- bug interno;
- constante generada corrupta detectada en `debug_assert`;
- violación imposible mediante API segura.

## 8.2 Determinismo

Debe ser idéntico:

- encoding;
- resultados;
- normalización;
- `FieldId`;
- certificados;
- vectores;
- código generado, salvo cabecera no semántica prohibida.

Los benchmarks no son deterministas y no forman parte de `FieldId`.

## 8.3 Timing

Estas fases no prometen tiempo constante. Cada operación publicará:

```rust
pub enum ScheduleKind {
    DataIndependent,
    DataDependent,
}
```

`FixedSchedule` solo elige kernels marcados `DataIndependent`; no transforma
automáticamente una inversión variable en constante.

## 8.4 Paralelismo

No se añade Rayon. Los métodos procesan un slice en el hilo llamante. Una capa
superior puede dividir:

```rust
values.par_chunks_mut(tile)
```

sin que el núcleo conozca el scheduler.

## 8.5 Compatibilidad

```rust
pub enum FieldCompatibility {
    Exact,
    SameCardinalityDifferentPresentation,
    DifferentCardinality,
}
```

`Exact` exige descriptor canónico idéntico. Una isomorfía entre
presentaciones requiere en el futuro un `BasisTransform` explícito.

# 9. Dependencias

## 9.1 Runtime

Objetivo:

- cero dependencias obligatorias en `no_std + portable`;
- ninguna dependencia de serialización en el hot path;
- `core::arch` para ISA;
- `alloc` solo para `PackedBatch` owned.

## 9.2 Generador

Dependencias opcionales:

- `serde`;
- `serde_json`;
- `toml`;
- `sha2`;
- una biblioteca de enteros grandes cuando empiece validación general de
  primos;
- `clap` solo si reduce complejidad del CLI; de lo contrario parser pequeño.

## 9.3 Desarrollo

- `proptest`;
- `criterion`;
- RNG determinista para tests;
- herramientas de assembly externas;
- Sage/NTL fuera del binario de producción.

# 10. CI

## 10.1 Jobs mínimos

| Job | Target | Propósito |
|---|---|---|
| `fmt-clippy` | host | estilo y lints |
| `portable-std` | x86-64 | tests completos |
| `portable-no-std` | target sin SO | compilación |
| `msrv` | Rust 1.89 | compatibilidad |
| `miri` | portable | wrappers y packed |
| `external-field-consumer` | crate fixture | factory, build script y tipo externo |
| `x86-pclmul` | hardware real | diferencial |
| `x86-vpclmul` | hardware real | diferencial y benchmark |
| `arm-pmull` | hardware real | diferencial y benchmark |
| `qemu-arm` | emulado | apoyo de corrección |
| `generated-clean` | host | regenerar y comprobar diff vacío |
| `oracle-vectors` | entorno matemático | vectores mantenidos |
| `asm-audit` | x86/ARM | instrucciones esperadas |

## 10.2 Seeds

Todo property test registrado en CI guarda:

- seed;
- campo;
- operación;
- versión;
- valor mínimo reducido.

Un fallo se convierte en vector de regresión.

# 11. Backlog de implementación

## Épica E0.1 - Paquete y contratos

- crear paquete;
- configurar `no_std`;
- crear errores;
- definir traits;
- crear ADR;
- compile tests de jerarquía.

## Épica E0.2 - Manifiesto e identidad

- parser;
- normalización;
- proyección de identidad;
- `FieldId`;
- `ArtifactId`;
- golden tests.

## Épica E0.3 - Validación binaria

- polinomios por limbs;
- gcd;
- exponenciación de $X$;
- Rabin;
- certificado;
- casos reducibles.

## Épica E0.4 - Planificador y emisor

- `ProductPlan`;
- `ReductionPlan`;
- `ExponentiationPlan`;
- verificador de planes;
- emisión transaccional;
- CLI.

## Épica E0.5 - Oráculos y harness

- formato de vectores;
- import Sage;
- import NTL opcional;
- tests generados;
- benchmark skeleton.

## Épica E1.1 - Representaciones concretas

- tres tipos;
- encoding;
- metadatos;
- operadores estándar.

## Épica E1.2 - Portable ancho/reducción

- `clmul64_reference`;
- escolar;
- reducción lenta;
- reducción por plan;
- diferenciales.

## Épica E1.3 - Algoritmos derivados

- cuadrado propio;
- `mul_by_x`;
- `pow`;
- inversión;
- Frobenius;
- traza/norma.

## Épica E1.4 - Batch portable

- `KernelSet`;
- `Engine::portable`;
- slice APIs;
- in-place;
- tails;
- benchmarks.

## Épica E2.0 - Factory binaria pública

- `BinaryFieldDefinition` y Builder público;
- adaptador de manifiesto para `build.rs`;
- typestate normalizado/validado/generado;
- ABI de codegen versionado;
- emisión de newtype, traits, metadata y portable batch;
- presets mantenidos regenerados por la misma factory;
- fixture de consumidor externo;
- determinismo, `no_std`, compile-fail y límites adversariales.

## Épica E2.1 - Motor

- capabilities;
- políticas;
- builder;
- selector;
- metadatos;
- errores.

## Épica E2.2 - x86

- PCLMUL escolar;
- PCLMUL Karatsuba;
- reducción;
- VPCLMUL;
- packing;
- auditoría.

## Épica E2.3 - ARM

- PMULL;
- reducción;
- batches;
- packing;
- auditoría.

## Épica E2.4 - Packed

- `PackingPlan`;
- `AlignedBuffer`;
- owned batch;
- vistas;
- conversiones;
- operaciones packed.

## Épica E2.5 - Rendimiento y selector

- benchmark matrix;
- registros por CPU;
- thresholds;
- puntos de equilibrio;
- gates de regresión.

# 12. Orden de ejecución

## Fase 0

Semanas orientativas 1-4:

1. paquete, traits y errores;
2. manifiesto, normalización e identidad;
3. validación binaria y certificados;
4. planes, CLI, vectores y harness.

No se empieza Fase 1 hasta congelar:

- schema v1;
- encoding de los dos campos iniciales;
- `FieldId`;
- IR de reducción.

## Fase 1

Semanas orientativas 5-10:

1. `Gf2_128V1`;
2. `Gf2_256HhV1`;
3. `Gf2_256AltV1`;
4. multiplicación/reducción;
5. cuadrado/inversión;
6. batch portable y compatibilidad.

Primero se termina un campo verticalmente. Después se prueba que el generador
elimina duplicación con los otros dos.

## Fase 2

Orden revisado después del cierre de Fase 1:

1. factory pública y generación estática de GF(2^m) externos;
2. optimizador portable estático para la mayoría de GF(2^m);
3. capabilities, `EngineBuilder` detectado y catálogo ampliado;
4. PCLMUL;
5. perfiles ISA externos verificados mediante ABI 3;
6. PMULL;
7. `PackedBatch` y vistas sobre storage aportado;
8. VPCLMUL y layouts persistentes;
9. thresholds, auditoría, CI multi-ISA y cierre.

PCLMUL y PMULL se desarrollan sobre el mismo conjunto de vectores. VPCLMUL no
bloquea la corrección del motor si no alcanza aún el rendimiento esperado.
Los campos externos conservan portable y reciben elegibilidad ISA estructural
solo después de validación/certificación. Corrección no implica selección
automática: esa decisión exige calibración por target.

# 13. Ejemplo de uso objetivo

## Escalar

```rust
use microfield::{
    CanonicalEncoding,
    Field,
    Gf2_256HhV1,
};

let a = Gf2_256HhV1::from_canonical(&a_bytes)?;
let b = Gf2_256HhV1::from_canonical(&b_bytes)?;

let c = a * b;
let s = c.square();
let inverse = s.invert();
let encoded = c.to_canonical();
```

## Batch AoS

```rust
use microfield::{
    Engine,
    EngineBuilder,
    ExecutionPolicy,
    Gf2_256HhV1,
};

let engine = EngineBuilder::<Gf2_256HhV1>::new()
    .policy(ExecutionPolicy::Throughput)
    .expected_batch(lhs.len())
    .build()?;

engine.mul_into(&mut out, &lhs, &rhs)?;
```

## Batch persistente

```rust
let mut a = PackedBatch::from_aos(&engine, &lhs)?;
let b = PackedBatch::from_aos(&engine, &rhs)?;
let mut out = PackedBatch::new(&engine, lhs.len())?;

engine.mul_packed_into(&mut out, &a, &b)?;
engine.square_packed_into(&mut a, &out)?;
a.unpack_into(&mut result)?;
```

El usuario nunca observa `__m256i`, PMULL ni SoA.

# 14. Matriz de trazabilidad

| Objetivo original | Elemento de esta especificación |
|---|---|
| campo exacto y reproducible | `CanonicalFieldDescriptor`, `FieldId` |
| validación | `ValidationEngine`, certificados |
| representación separada | tipos concretos y traits internos |
| backend separado | `KernelCatalog`, `KernelSet` |
| batch de primera clase | `Engine`, `PackedBatch` |
| portable como oráculo | `binary::reference` y `backend::portable` |
| no dispatch escalar | implementación `Field` estática |
| detección una vez | `EngineBuilder` |
| PCLMUL/VPCLMUL | `backend::x86` |
| PMULL | `backend::aarch64` |
| encoding estable | `CanonicalEncoding` |
| unsafe confinado | `backend` y `packed::aligned` |
| pocos crates | un único paquete Cargo |
| reutilización jerárquica | traits, composición y tipestate |
| campos externos sin hardcode | `BinaryFieldFactory` y codegen versionado |

# 15. Decisiones que no deben reabrirse durante Fases 0-2

1. No dividir el paquete en crates por backend.
2. No introducir grafos o firmas algebraicas.
3. No añadir backend al tipo de elemento.
4. No usar dispatch en operadores escalares.
5. No serializar layouts internos.
6. No llamar «canónica» a una reducción de bytes.
7. No afirmar que $X$ es primitivo sin certificado.
8. No depender de `std::simd` experimental.
9. No usar AVX-512 en el alcance inicial.
10. No añadir paralelismo por hilos al núcleo.
11. No convertir la factory estática en un contexto dinámico.
12. No exponer catálogos raw para habilitar campos externos.
13. No aceptar un kernel solo porque contiene intrinsics.
14. No publicar `unsafe` como API de usuario.

# 16. Riesgos y mitigación

## Demasiada lógica en un solo paquete

Mitigación: módulos con dependencias unidireccionales, visibilidad `pub(crate)`
y owners claros. Un paquete no implica acoplamiento circular.

## Duplicación entre tipos generados

Mitigación: IR y plantillas comunes, traits internos y generación revisable.
No se intenta expresar tamaños anchos con const generics inestables.

## Factory pública bloquea el ABI interno

Mitigación: la salida usa un ABI de codegen versionado y estrecho. El contrato
público describe matemáticas y encoding; `KernelSet`, punteros, limbs internos
y wrappers ISA siguen privados. Fixtures de versiones N/N-1 detectan roturas
antes de estabilizar una revisión del ABI.

## Generador incorrecto

Mitigación: reducción lenta independiente, Sage/NTL, certificados, golden
vectors y verificación del IR.

## Dispatch costoso

Mitigación: una selección por `Engine`, una indirección por lote y benchmark
directo contra kernel.

## Packing no rentable

Mitigación: AoS sigue disponible, `PackedBatch` persiste y el selector publica
su punto de equilibrio.

## Backend disponible pero lento

Mitigación: metadata y thresholds por región. Disponibilidad no implica
selección.

## `unsafe` divergente

Mitigación: mismos planes generados, wrappers seguros, differential tests,
sanitizers y auditoría de ensamblador.

# 17. Resultado al finalizar Fase 2

El repositorio contendrá un único paquete coherente capaz de:

1. describir un campo binario;
2. normalizar y validar el descriptor;
3. certificar irreducibilidad;
4. generar desde un crate consumidor tipos externos nominales;
5. generar constantes, planes, metadata y código portable;
6. codificar elementos de forma canónica;
7. ejecutar suma, producto, cuadrado, potencia e inversa;
8. procesar slices sin asignaciones;
9. empaquetar lotes persistentes;
10. seleccionar portable, PCLMUL, VPCLMUL o PMULL;
11. demostrar igualdad bit a bit;
12. publicar la región de rendimiento de cada kernel;
13. compilar tipos generados y portable en `no_std`.

Al cierre de Fase 2 la biblioteca quedó preparada para Fase 3, pero todavía no
prometía:

- inversión batch;
- Itoh-Tsujii optimizado;
- Horner batch;
- campos primos;
- contextos dinámicos;
- firmas algebraicas.

La sección 18 materializa después inversión batch, Itoh–Tsujii verificado y
Horner. Campos primos, contextos dinámicos y firmas permanecen en fases
posteriores.

# 18. Fase 3 implementada: algoritmos derivados

La Fase 3 queda materializada sobre la arquitectura cerrada en Fase 2:

1. IR v4 de inversión Itoh–Tsujii con verificación simbólica obligatoria;
2. `BitMask` compacta y vista prestada;
3. inversión batch tolerante a cero con un único inverso escalar;
4. workspace tipado, reutilizable y naturalmente alineado;
5. scans prefijo/sufijo, inclusivos y exclusivos;
6. Horner para un polinomio/muchos puntos y muchos polinomios/un punto;
7. layout de coeficientes explícito y sin transposición oculta;
8. `mul_add_into` como operación derivada;
9. tablas de potencias de base fija, prestadas y owned;
10. planes inmutables ligados a `FieldId`, `BackendId` y longitud.

Las rutas prestadas funcionan en `no_std` sin `alloc`, no detectan CPU, no
crean hilos y no amplían el inventario `unsafe`. Toda validación ocurre antes
de modificar la salida. La evidencia, los tests y los límites están en
`docs/microfield/phase-3-plan.md`.

# 19. Enriquecimiento vinculante de Fase 6

Fase 6 ya no queda limitada a firmas algebraicas nuevas. Comenzará por la
completación, corrección y extensión de todo el código legado mantenido,
aplicándolo sobre los campos y engines de `microfield`. Se congelarán primero
vectores y semántica; se conservará compatibilidad solo cuando esté demostrada,
y se retirarán afirmaciones criptográficas o probatorias injustificadas.

Esta ampliación sustituye expresamente la decisión `ARCH-109` de la
especificación externa de Fases 3–7. La canonización entra en el programa, pero
no en el núcleo algebraico: el dominio de grafos dependerá de `microfield`, y
`field`, `kernel`, `backend` y los encodings canónicos de elementos no
dependerán de grafos.

La fase incluye también un track de análisis estructural de grafos:

1. especificación exacta del modelo de grafo y su encoding;
2. algoritmo determinista de etiquetado y firma lineal por ronda;
3. perfiles sobre F251 y cualquier campo estático generado;
4. huella híbrida de firma algebraica más SHA-256 invariante;
5. pruebas masivas de renumeración y casos estructuralmente adversariales;
6. canonización exacta únicamente como perfil optativo con presupuesto.

La corrección no dependerá de una ausencia supuesta de colisiones. La igualdad
de firmas, incluso acompañada por SHA-256, seguirá siendo una huella y no una
prueba de isomorfismo. El desarrollo detallado está trazado en
`docs/microfield/phases-3-7-roadmap.md` y
`docs/microfield/phase-6-fast-graph.md`.

# 20. Fase 4 implementada: campos primos

La arquitectura soporta ya característica prima sin modificar `Field`, el
elemento binario ni los algoritmos derivados. Se mantienen tres tipos
nominales:

1. `Fp251V1`, residuo canónico de un byte y backend AVX2;
2. `FpGoldilocks64V1`, residuo canónico de palabra y reducciones Solinas y
   Barrett verificadas;
3. `Fp256GenericV1`, primo determinista de 256 bits en Montgomery CIOS y
   primera instancia del backend BMI2 radix-64 genérico.

`PrimeField`, `SquareRootField`, `PrimeRepresentationKind`, los planes de
reducción y `PrimeKernelMetadata` forman contratos segregados. Los limbs,
productos anchos, valores Montgomery y estados lazy siguen privados. El
producto ancho se separa de la reducción mediante un trait interno estático;
no existe `dyn Trait` ni dispatch en operaciones escalares.

Cada campo posee `FieldId`, `ArtifactId`, certificado y bundle. El primo
genérico se reproduce desde una semilla pública; Pocklington se verifica dentro
del runtime sin depender de Sage. SageMath aporta un segundo oráculo y un corpus
determinista para suma, resta, producto, cuadrado e inversa.

La selección conserva la política de estabilidad: AVX2 para 251 entra en
`Auto` desde 64 elementos porque gana en la región medida. La extensión
F4.6-SIMD incorpora Goldilocks AVX2 de cuatro lanes, automático desde 4 tras
medir producto, square y suma; ofrece además factories AVX2 explícitos para
perfiles externos canónicos `u8`/`u16`. El puerto
público-oculto `VerifiedPrimeMontgomery64Field<N, 2N>` y la estrategia opaca
`VerifiedPrimeIsaStrategy` permiten formar candidatos BMI2 seguros para tipos
mantenidos o generados externamente con 64, 128, 192, 256 bits de almacenamiento
y anchuras posteriores, sin duplicar el producto `MULX`. Esto prueba
compatibilidad, no velocidad: la promoción permanece ligada a cada campo y
región medida. BMI2 de 256 bits es correcto y forzable, publica
`Fixed` tras sustituir carry y corrección por recorridos completos y selección
branchless. `FixedSchedule` lo acepta, pero permanece fuera de `Auto` porque la
remedición no acredita una ventaja estable superior al 3 %. IFMA y un backend
primo AArch64 no se anuncian sin hardware, cobertura y medición reproducible.

VPCLMUL ejecuta dos pares independientes por iteración para mejorar ILP. Las
mejoras largas observadas en GF(2²⁵⁶) no bastan para desplazar PCLMUL de forma
estable, por lo que sigue fuera de `Auto`. La generalización AVX2 externa no
presupone zero-copy: Fp251 conserva el kernel especializado porque convertir
cada valor mediante el bridge resultó mucho más lento. Compatibilidad,
representación y promoción medida permanecen como decisiones separadas.

La factory TOML v1 no se amplía retrospectivamente: acepta solo campos
binarios. La factory de primos externos, junto a assurance demostrado/probable,
lock y contextos, abrirá la Fase 5 tras el cierre de F4.7-PACKED-SIMD. El plan y el
informe autoritativos están en
`docs/microfield/phase-4-plan.md` y
`docs/microfield/phase-4-final-report.md`. La extensión está especificada en
`docs/microfield/phase-4-6-plan.md`, `docs/microfield/phase-4-6-report.md` y ADR
0024.

## 20.1 Extensión completada F4.7-PACKED-SIMD

Antes de abrir la generación prima externa de Fase 5 se ha desarrollado un
puente persistente por lanes. El bridge AVX2 directo de F4.6 sigue siendo el
fallback seguro para `&[F]`, mientras `PackedBatch<F>` puede almacenar
internamente residuos `u8`, `u16` o `u32` y ejecutar una cadena sin volver a
convertir cada elemento.

La arquitectura añade un ABI `kernel::packed` neutral, storage tagged privado
y codecs estáticos `F ↔ Lane` usados únicamente al entrar y salir. No se
reinterpretará memoria externa ni se expondrán lanes. Fp251 y Goldilocks
conservan sus kernels AoS directos especializados. Los candidatos externos
siguen fuera de `Auto`, aunque el pipeline reutilizado sea favorable.

El orden ejecutado ha sido:

1. baseline y ADR propuesto;
2. ABI packed y metadata de storage;
3. owned/vistas alineadas para `u8`/`u16`;
4. migración de kernels genéricos sin codecs dentro del loop;
5. candidato `u32` con Barrett vectorial;
6. cinco operaciones packed y pipelines repetidos;
7. calibración, Miri/ASan/ASM, compatibilidad e informe final.

`u64` genérico, AArch64 primo, AVX-512 e IFMA permanecen fuera. El plan
autoritativo, la matriz de tests y los gates cuantitativos están en
`docs/microfield/phase-4-7-plan.md`; ADR 0025 está aceptado después de superar
el prototipo, la corrección y la medición. El resultado completo está en
`docs/microfield/phase-4-7-final-report.md`.

# 21. Fase 5 implementada: generación externa y contextos dinámicos

La apertura a campos externos queda completada sin convertir el núcleo en una
factory virtual. El schema binario v1 permanece cerrado y se añade un schema
primo v1 separado. `ValidationAssurance` distingue prueba determinista de
probable primalidad; únicamente `Proven` autoriza fuente Rust.

`PrimeFieldFactory` genera tipos nominales canónicos `u8`, `u16`, `u32` o
Montgomery radix-64 según el módulo. Cada paquete contiene descriptor,
certificado, plan, vectores, fuente, `microfield.lock`, índice y README. La
publicación es transaccional y la caché inmutable verifica digests en lectura.
Los bridges AVX2/BMI2 son seguros y explícitos hasta calibración por campo.

La ruta runtime añade `DynField`, `DynElement`, storage inline hasta ocho
limbs, `DynBatch` y `DynEngine`. Rabin valida campos binarios; Miller–Rabin
determinista o Pocklington prueba primos; probable permanece etiquetado. Los
checks nominales se amortizan por lote. El puente `generate_static` reejecuta
la certificación y comprueba igualdad de `FieldId`.

SageMath 10.7 ha validado los cuatro perfiles externos de aceptación. La
arquitectura, gates y límites se congelan en
`docs/microfield/phase-5-plan.md`, ADR 0026 y
`docs/microfield/phase-5-final-report.md`.

# 22. Fase 6 implementada antes de canonización: firmas estructurales

F6.0–F6.8 completan el inventario, corrección, generalización y migración de la parte algebraica
del legado. `GaloisSignature256` preserva su layout y encoding, pero delega en
`Gf2_256HhV1`; ya no existe una segunda implementación del mismo campo.
`FiniteField`, `TopoHasher` y los agregadores antiguos permanecen como adapters
de compatibilidad.

El módulo `structural` introduce una API genérica sobre campos estáticos:

1. `StructuralEncoder<F>` y encoders canónico, binario y primo;
2. `EncoderId` y `SignatureId` para ligar campo, encoder, ley y parámetros;
3. `AdditiveSignature` para suma y paridad con contador;
4. `SequenceSignature` para Horner con longitud y concatenación exactas;
5. `MultisetSignature` para producto con cardinalidad y factores cero;
6. `TrackedSequence` y `TrackedMultiset` cuando se requiere orden o pertenencia
   reales;
7. `AlgebraicResidual` para relaciones inversas, sin llamarlas pruebas;
8. wire `MFSG` schema 1 con parsing estricto;
9. ingestión masiva transaccional y ruta inline sin asignaciones.
10. ingestión directa de elementos para consumidores que ya trabajan en `F`;
11. prueba de extensibilidad mediante un GF(2⁹) externo generado en build time;
12. adapters `dynamic-fields` sobre `DynField`, separados de la ruta estática y
    wire-compatibles para un mismo `FieldId`;
13. `BidirectionalSequenceSignature` con Horner forward/reverse;
14. `MultiEvaluationMultisetSignature<F, E, K>` con puntos distintos y conteo
    de factores cero independiente por evaluación;
15. equivalentes dinámicos con puntos runtime validados.

El propósito queda fijado: son hashes homomórficos no criptográficos. Una
evaluación pequeña puede capturar leyes útiles y combinar particiones, pero no
es inyectiva ni autentica el historial. `crypto_mode` solo conserva código
fuente antiguo y no promete tiempo constante o seguridad.

La auditoría completa está en `docs/microfield/phase-6-legacy-audit.md`, la
arquitectura ejecutada en `docs/microfield/phase-6-pre-canon-plan.md` y la
decisión semántica en ADR 0027 y la frontera de generalización en ADR 0028.
F6.G0–G2 añaden posteriormente `IncidenceGraph`,
`FastGraphLabeler<F, E, K>`, el perfil F251, campos externos generados y el
canal híbrido SHA-256 documentado en
`docs/microfield/phase-6-fast-graph.md`. El nombre del canonizador legado
permanece como compatibilidad, pero su implementación delega en el motor F251
nuevo; la búsqueda exacta no entra en el hot path.

# 23. F6.G0–G6: motor rápido, incrementalidad y canonización exacta acotada

La discusión de grafos corrige la prioridad inicial: la búsqueda canónica
potencialmente exponencial no será el flujo predeterminado. El producto central
es `FastGraphLabeler<F, E, K>`, un etiquetador invariante por renumeración con
coste `O(K R (V + I))`.

`IncidenceGraph` normaliza en CSR un multigrafo dirigido relacional. Etiquetas,
roles, dirección, bucles y multiplicidades se conservan exactamente. Las
hiperaristas son nodos auxiliares y cuestan linealmente en su aridad. El adapter
`from_legacy_topology` permite migración sin seguir expandiendo cláusulas a
cliques.

F251 se conserva como especialización prioritaria mediante
`F251GraphLabeler<K>`. `FastGraphLabeler` también acepta campos binarios,
primos mantenidos y cualquier tipo estático producido por la factory, con
parámetros derivados o explícitos ligados a `GraphSignatureId`.

El perfil `Fast` conserva los productos y contadores de cero de cada ronda;
`combine_disjoint` produce exactamente la firma de la unión disjunta sin
reprocesar sus componentes. Los perfiles adaptativos no exponen esa ley porque
pueden detenerse bajo calendarios diferentes.

Las capacidades públicas son:

1. `Fast`, con rondas fijas y predecibles;
2. `Robust`, con estabilización de partición y máximo estricto;
3. `try_canonicalize`, que solo emite forma exacta si todas las clases son
   unitarias y devuelve `SymmetryRemaining` en caso contrario;
4. `diagnose_degeneracy`, que separa aliasing finito y ambigüedad exacta local;
5. `MultiFieldGraphEvidenceBuilder`, que identifica conjuntos heterogéneos de
   evidencia sin afirmar isomorfismo;
6. `canonicalize_exact`, opt-in y con presupuesto explícito.

`analyze_hybrid` añade un segundo canal SHA-256 calculado sobre histogramas
invariantes de todas las rondas, etiquetas exactas y relaciones refinadas. No
se calcula sobre índices ni sobre la firma de campo sola. La pareja reduce
colisiones globales cuando los descriptores adicionales difieren, pero no se
presenta como prueba de isomorfismo.

Las primeras mediciones release con F251, tres lanes y cuatro rondas alcanzan
entre 28,5 y 32,7 M incidencias-ronda/s para 16.384 vértices. El modo híbrido
queda entre 8,53 y 9,34 M/s y GF(2²⁵⁶) alrededor de 155–157 K/s en el mismo
flujo. Son cifras locales; establecen F251 como candidato prioritario y
SHA-256 como opt-in.

F6.G3 queda ejecutado: `PreparedGraph` y `GraphWorkspace` separan preparación
de ejecución, precalculan constantes afines y permiten vistas prestadas sin
asignaciones en el camino secuencial caliente. La comparación completa muestra
que SoA+AVX2 acelera el caso monohilo, mientras AoS+Rayon domina en grafos
grandes del host auditado; ambas estrategias son explícitas y producen bytes
idénticos. `CellularGaloisCanonizer` se ha convertido en fachada sobre el mismo
motor F251 y las pruebas de su antigua recurrencia se han sustituido por
contratos del puente mantenido. F6.G4 queda también ejecutado:
`IncrementalGraphState` conserva las capas de ronda, audita el nuevo CSR,
recalcula únicamente el cono afectado y publica firma, partición y componentes
de forma transaccional. Los benchmarks locales observan aproximadamente
`2,0–2,5×` para una etiqueta y `1,6–2,1×` para una arista frente a recomputación
completa.

F6.G5 queda ejecutado con un diagnóstico exacto que distingue aliasing de campo
de ambigüedad 1-WL, un umbral público de alta regularidad y bundles multi-campo
ligados a `GraphEvidenceProfileId`. El corpus prueba desde seis vértices que
`C6` y `C3 ⊔ C3` comparten firma local en F251, GF(2²⁵⁶) y SHA híbrido sin ser
isomorfos. SageMath 10.7 confirma 35 pares entre 6 y 40 vértices; 128
normalizaciones aleatorias y los universos completos de cuatro y cinco
vértices aportan oráculos diferenciales adicionales. El par fuertemente
regular Shrikhande/torres 4×4 cubre además la degeneración extrema de parámetros
`(16,6,2,2)`.

F6.G6 queda ejecutado mediante `canonicalize_exact`: ruta rápida para una
partición discreta y DFS iterativo de individualización–refinamiento para las
simetrías. `CanonicalSearchBudget` limita nodos y estado retenido. Solo un árbol
completo produce `Exact`; cualquier límite produce `BudgetExhausted` sin forma
parcial. Esta búsqueda es opt-in y no modifica `analyze`, batch ni incremental.
Con ello la Fase 6 queda cerrada. El informe consolidado está en
`docs/microfield/phase-6-final-report.md`; el detalle de los dos últimos cortes
está en `docs/microfield/phase-6-g5-g6-final-report.md`.

# 24. F6.G7: corrección del discriminador global y corpus externo

El cierre anterior se reabre porque la colisión `C6`/`C3 ⊔ C3`, aunque estaba
diagnosticada, hacía insuficiente la firma local como producto principal de
clasificación. Se mantiene `FastGraphSignature` v1 para compatibilidad,
composición y latencia mínima, y se introduce un perfil v2 recomendado.

`GlobalGraphProfile` incorpora componentes débiles, SCC, tamaños, tipos y
etiquetas, relaciones/roles, grados, multiplicidades, bucles, soporte simple y
rango cíclico. La serialización exacta interna etiquetas y relaciones una vez;
la igualdad no depende del SHA-256 que la identifica. Así `C6` y `C3 ⊔ C3` se
separan antes de cualquier búsqueda.

`GraphDiscriminationPolicy::Adaptive` añade triángulos y `K4` para grafos con
particiones altamente regulares. Una cota de trabajo invariante decide la
admisión; si no cabe, `SkippedBudget` no contiene resultados parciales. Esto
separa Shrikhande de torres 4×4 (`K4=0` frente a `K4=8`). Comparar perfiles v2
solo devuelve `Different` o `Indistinguishable`.

`canonicalize_exact` descompone componentes débiles, consume el presupuesto de
nodos restante por componente, ordena formas exactas y publica solo al
completar toda la unidad de trabajo.

La prueba externa queda separada de `cargo test`: un manifiesto con URL,
SHA-256, licencia y cita alimenta un fetcher atómico/offline. El corpus cubre
1.253 clases del Graph Atlas hasta siete vértices, 188 moléculas MUTAG, la red
dirigida etiquetada email-Eu-core y el hipergrafo biológico diseasome. La Fase 6
se vuelve a cerrar únicamente tras superar este corpus, las suites locales,
Clippy/rustdoc y los benchmarks de coste aislado. El detalle está en
`docs/microfield/phase-6-g7-final-report.md` y ADR 0030.

# 25. F6.V: validación científica y aplicada antes de publicación

F6.G7 cerró la implementación de firmas y grafos. F6.V1–V6 implementan el
programa de validación bloqueante definido en
`docs/microfield/phase-6-validation-plan.md`; su primera evidencia se conserva
en `docs/microfield/phase-6-validation-final-report.md` y
`validation/f6/results/semantic-v1.json`.

El programa exige:

1. leyes exhaustivas y metamórficas para cada firma;
2. catálogo reproducible de colisiones, parámetros y límites;
3. aplicaciones de agregación, reconciliación y secuencias comparadas con
   baselines adecuados;
4. corpus de grafos exhaustivo, adversarial y multiformato con oráculo exacto;
5. tasas de indistinguibilidad por escalón v1/híbrido/v2/motivos/multi-campo;
6. curvas de coste incremental, memoria, latencia y búsqueda exacta;
7. repetición en varias familias x86-64 y ARM64;
8. clasificación final de cada vía como validada, primitiva, experimental o
   descartada.

La reconciliación incluye ya recuperación acotada con distancia desconocida:
63.232 pares exhaustivos hasta diferencia seis se recuperan y verifican. Sigue
como `ValidatedPrimitive` hasta disponer de factorización escalable y API
pública. El corpus de 12.346 clases de orden ocho deja 454 grafos ambiguos en
v1 y 46 tras motivos adaptativos; SHA híbrido y multi-campo local no reducen la
regularidad. En grafos, una igualdad continúa significando
`Indistinguishable`; solo un oráculo exacto completado certifica la forma.

La implementación V1–V6 queda cerrada. La publicación continúa bloqueada por
la evidencia multi-microarquitectura y los baselines externos enumerados en el
informe, no por falta de infraestructura.

# 26. F6.G15: preparación para consumo interno

Tras G8–G14, la biblioteca se considera un release candidate interno
condicionado: firmas y filtros son utilizables como evidencias negativas, y
`Microcanon` es utilizable como autoridad exacta siempre que `Inconclusive` se
propague. Todavía no se aprueba usar una firma rápida como clave persistente ni
integrar adapters de dominio sin schema explícito.

G15 queda separada de publicación. Las firmas homomórficas son producto de
primer nivel y grafos/canonización un vertical adicional. Sus hitos son:

1. inventario de campos, encoders, firmas, protocolos y grafos;
2. API soportada para suma, secuencias, multisets, multievaluación y tracking;
3. wire/snapshots, agregación distribuida, streaming y reconciliación;
4. deltas versionados y transaccionales por ley, sin claims criptográficos;
5. aplicaciones sobre archivos, bases de datos y árboles jerárquicos;
6. perfiles de campo por característica, cardinalidad, ley, K y backend;
7. perfiles de grafos, DAG exacto y adapter de cliques/subredes;
8. cierre de corpus, oráculos y colisiones residuales de ambos tracks;
9. property/fuzz tests de firmas, deltas, wires, parser y `GraphDelta`;
10. SLO por workload, consumidor, runbook y artefacto reproducible go/no-go.

La salida será `ReadyForInternalUse`, `Conditional` o `NotReady`; nunca una
aprobación narrativa. Quedan fuera licencia, crates.io, semver 1.0, claims
multi-CPU, SLA externo y equivalencia científica general. El desarrollo
completo, gates y Definition of Done están en
`docs/microfield/phase-6-g15-internal-readiness-plan.md`. La auditoría de la
capacidad existente y la frontera viable de deltas está en
`docs/microfield/phase-6-signature-delta-audit.md`.

El cierre transversal de campos, firmas, deltas, reconciliación, grafos,
calidad y operabilidad hasta una release candidate técnica queda consolidado en
`docs/microfield/release-candidate-readiness-plan.md`.

RC.0 y RC.1 quedan completados localmente: el paquete separa `signatures`,
`dynamic-signatures`, `graph` y `legacy`, y el inventario ejecutable valida
campos mantenidos, un campo externo generado y su presentación runtime. El
informe está en `docs/microfield/rc-0-rc-1-implementation-report.md`.

RC.2 queda completado localmente con builders estáticos/runtime, perfiles de
campo y evaluación, trait compacto sellado y snapshots exactos `MFTS` para
`TrackedSequence`/`TrackedMultiset`. La evidencia está en
`docs/microfield/rc-2-signature-api-report.md`.

RC.3 queda completado localmente con envelopes `MFDE`, deltas segregados por
ley, estado revisionado, aplicación preflight/candidate/commit y journal
persistible `MFDJ`. Las campañas aleatorias comparan cada transición con
rebuild y el replay repetido se reconoce por `DeltaId`. La evidencia está en
`docs/microfield/rc-3-delta-core-report.md`.

RC.4 queda completado localmente con chunking fijo identificado, framing
`MFFC`, árbol ordenado `HomomorphicSummaryTree`, edits locales O(k log n),
fallback completo al cambiar fronteras y checkpoints exactos `MFST`. La
evidencia está en `docs/microfield/rc-4-summary-tree-report.md`.

RC.5 queda completado localmente con schema y filas `MFRW`, tabla particionada,
before/after images, transacciones `MFTX`, log `MFTL` y reconciliación pública
`MFRS`. V1 declara claves primarias únicas y reconciliación de conjuntos sin
multiplicidad. La evidencia está en
`docs/microfield/rc-5-database-reconciliation-report.md`.

RC.6 queda completado localmente con `CanonicalGraphDag`, snapshot persistente
`MFGD`, restauración que vuelve a canonizar cada nodo, dependencias acíclicas y
reutilización condicionada a igualdad de bytes canónicos completos. Los
adapters separan subred inducida —pérdida de frontera explícita—, subred cerrada
—pérdida rechazada— y clique relacional dirigido. `GraphDeltaUpdateReport`
expone por separado cambios de labels y topología, pero ambos exigen una nueva
canonización exacta antes de publicar identidad. La evidencia está en
`docs/microfield/rc-6-graph-dag-report.md`.

# Referencias técnicas

1. Rust Project,
   [`_mm_clmulepi64_si128`](https://doc.rust-lang.org/core/arch/x86_64/fn._mm_clmulepi64_si128.html),
   intrinsic PCLMUL estable.
2. Rust Project,
   [`_mm256_clmulepi64_epi128`](https://doc.rust-lang.org/core/arch/x86_64/fn._mm256_clmulepi64_epi128.html),
   VPCLMULQDQ, estable desde Rust 1.89.
3. Rust Project,
   [`vmull_p64`](https://doc.rust-lang.org/stable/core/arch/aarch64/fn.vmull_p64.html),
   producto polinómico AArch64.
4. Rust Project,
   [`is_x86_feature_detected!`](https://doc.rust-lang.org/stable/std/arch/macro.is_x86_feature_detected.html),
   detección x86 en runtime.
5. Rust Project,
   [`is_aarch64_feature_detected!`](https://doc.rust-lang.org/stable/std/arch/macro.is_aarch64_feature_detected.html),
   detección AArch64 en runtime.
6. Rust Project,
   [`std::simd`](https://doc.rust-lang.org/std/simd/),
   API todavía experimental en Rust 1.97.
