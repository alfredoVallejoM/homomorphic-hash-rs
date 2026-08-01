# Arquitectura

## Capas de runtime

```mermaid
flowchart LR
    Field[field / id / error]
    Binary[binary]
    Kernel[kernel]
    Backend[backend]
    Generated[generated]
    Engine[engine]

    Field --> Binary
    Field --> Kernel
    Binary --> Backend
    Kernel --> Backend
    Field --> Generated
    Binary --> Generated
    Kernel --> Generated
    Backend --> Generated
    Kernel --> Engine
```

Las flechas significan «puede depender de». `Engine` no conoce backends
concretos. La raíz de composición interna construye la estrategia portable y el
motor conserva una referencia a su tabla estática; ni el tipo de campo ni el consumidor
entregan punteros.

## Generador

```mermaid
flowchart LR
    CLI[CLI adapter] --> UseCases[use_cases]
    FS[filesystem adapter] --> Ports[ports]
    Sage[Sage adapter] --> Ports
    UseCases --> Model[model / typestate]
    UseCases --> Ports
```

Los casos de uso dependen de interfaces de persistencia y oráculo. El binario
compone adaptadores concretos; la lógica de validación no importa `std::fs`,
argumentos CLI ni procesos externos.

## Reglas verificables

- `field` no importa `binary`, `kernel`, `engine` ni `backend`.
- `binary` no importa `engine` ni `backend`.
- `engine` no importa módulos de arquitectura.
- `generated` no depende de `spec`.
- `spec` solo existe con `generator`.
- Todo runtime portable compila con `no_std`.
- La Fase 1 compila con `forbid(unsafe_code)`.

## Flujos

Escalar:

```text
Gf2_128V1 / Gf2_256HhV1 / Gf2_256AltV1
  → BinaryFieldImpl
  → Polynomial128<TAIL> / Polynomial256<TAIL>
  → producto carry-less / cuadrado dedicado compartidos
  → reducción const-generic con tail certificado
  → resultado
```

El value object vive en `generated`; `binary` concentra algoritmos
independientes de API. Cada tipo aporta únicamente identidad nominal,
metadatos y una estrategia estática privada. El macro interno elimina
boilerplate de delegación, pero no genera matemáticas distintas por campo.

Batch:

```text
PortableField generado → composición segura → KernelSet privado estático
                                             ↓ selección única
EngineBuilder → Engine → validación → una llamada indirecta → backend portable
```

`kernel` define el ABI neutral y metadatos; `backend::portable` implementa los
bucles; la raíz del crate compone ambos; `engine` solo selecciona, valida y
delega. Los presets conservan su catálogo sellado como frontera para futuros
slots ISA, pero la ruta portable no exige que un campo externo simule ser un
preset mantenido.

Generación:

```text
FieldManifest → NormalizedManifest → ValidatedFieldSpec
              → GenerationPlan → GeneratedArtifacts
```

## Extensión implementada en H2.1

H2.1 expone una fachada de factory sobre el pipeline, no el modelo interno:

```mermaid
flowchart LR
    Manifest[Manifest o Builder] --> Factory[BinaryFieldFactory]
    Factory --> Validate[Normalizar + Rabin + planes]
    Validate --> Package[GeneratedFieldPackage]
    Package --> Build[build.rs / OUT_DIR]
    Build --> Type[Tipo nominal externo]
    Type --> Portable[Scalar + batch portable]
```

El tipo externo se genera antes de compilar y no contiene contexto runtime. La
factory usa `std`; el módulo resultante conserva `no_std`, limbs privados y
dispatch escalar estático. `KernelSet` y la elegibilidad ISA permanecen bajo
control interno. El fixture externo compila campos de grados 9 y 233 y actúa
como prueba end-to-end de esta frontera.
