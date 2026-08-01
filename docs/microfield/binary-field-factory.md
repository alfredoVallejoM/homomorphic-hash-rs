# Guía de la factory de campos binarios

## Modelo

La factory genera tipos Rust estáticos para GF(2^m). No devuelve valores
heterogéneos ni contextos runtime. H2.1 admite característica dos, base
polinómica, encoding little-endian y grados 2..=4096. El polinomio debe ser
mónico, incluir el término independiente y superar Rabin.

## Configuración Cargo

```toml
[dependencies]
microfield = { path = "../microfield", default-features = false, features = ["portable"] }

[build-dependencies]
microfield = { path = "../microfield", default-features = false, features = ["generator"] }
```

Las features de build y runtime están separadas con resolver Cargo 2. El binario
final no incorpora parser TOML, SHA-256, Sage ni adaptadores de filesystem.

## Generación desde manifiesto

`fields/gf2_233_custom.toml`:

```toml
schema_version = 1

[field]
name = "gf2_233_custom"
characteristic = 2
degree = 233

[field.basis]
kind = "polynomial"
coefficient_order = "ascending"

[field.modulus]
nonzero_exponents = [233, 74, 0]

[field.encoding]
byte_order = "little"
bit_order = "lsb0"
canonical_bytes = 30

[build]
limb_bits = 64
product_strategies = ["schoolbook"]
reduction_style = "generated_fold"
requested_backends = ["portable"]
```

`build.rs`:

```rust
use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=fields/gf2_233_custom.toml");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    microfield::generator::BinaryFieldFactory::from_manifest(
        "fields/gf2_233_custom.toml",
    )
    .expect("manifest valid")
    .generate()
    .expect("módulo irreducible")
    .emit_rust(output)
    .expect("publicación atómica");
}
```

`src/lib.rs`:

```rust,ignore
#![no_std]
include!(concat!(env!("OUT_DIR"), "/gf2_233_custom.rs"));
```

El tipo resultante se llama `Gf2_233Custom`. Implementa `Field`, `Square`,
`Invert`, `Pow`, `CanonicalEncoding`, `ExtensionField`,
`BinaryPolynomialField`, `StaticField`, operadores y formateo. También puede
usarse como `Engine::<Gf2_233Custom>::portable()` cuando la feature `portable`
está activa.

## Builder

```rust,ignore
let package = BinaryFieldFactory::builder()
    .name("gf2_233_custom")
    .degree(233)
    .modulus_exponents(vec![233, 74, 0])
    .build()?
    .generate()?;
```

El nombre debe ser `snake_case` ASCII estable. Los exponentes pueden llegar en
cualquier orden; duplicados, exponentes fuera de grado y polinomios reducibles
se rechazan. `maximum_degree` permite aplicar una cota local más estricta, pero
nunca elevar el techo del esquema.

## Actualizaciones y reproducibilidad

La misma versión de Microfield, manifiesto y build normalizado produce los
mismos bytes. La fuente comprueba el ABI de codegen en compilación. Se
recomienda no versionar archivos de `OUT_DIR`; sí deben versionarse el
manifiesto y, cuando corresponda, certificados/vectores externos.

Los campos con el mismo grado no son intercambiables. `FieldId` identifica la
semántica matemática; `ArtifactId` identifica el perfil generado y el digest de
bundle autentica los bytes exactos de los artefactos.

La fuente actual usa ABI de codegen 2; el runtime acepta ABI 1..=2 para que un
archivo generado por la versión anterior siga compilando durante la ventana de
compatibilidad. `package.portable_optimization()` permite registrar la clase
de grado y las estrategias elegidas sin exponer limbs ni punteros.

La optimización es automática y reproducible. Grados alineados con tail bajo
usan fold por palabras; módulos dispersos, fold por términos; módulos densos,
un tail empaquetado. Todos usan cuadrado dedicado e inversión Itoh–Tsujii. No
hay detección ni selección dentro de las operaciones. Véase
[`portable-optimizer.md`](portable-optimizer.md).

El fixture ejecutable de referencia está en
`crates/microfield/test-fixtures/external-consumer`; contiene campos de grado
9, grado 10 con módulo denso y grado 233 validado con Sage.
