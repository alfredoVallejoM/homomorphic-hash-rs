# Fase 5 — generación externa y contextos dinámicos

## Estado

Implementada. La Fase 5 mantiene separados tres productos: definición
matemática, bundle Rust estático y contexto runtime. No carga código desde un
manifiesto, no incorpora JIT y no modifica el camino escalar de los campos
mantenidos.

## Hitos ejecutados

| Hito | Entrega | Gate |
|---|---|---|
| F5.0 | decisiones, schema primo v1 y `ValidationAssurance` | binario v1 permanece inalterado |
| F5.1 | prueba determinista `u64` y Pocklington multi-limb | probable no emite Rust |
| F5.2 | factory prima nominal | perfiles `u8`/`u16`/`u32`/Montgomery compilan |
| F5.3 | bundle, `microfield.lock` y emisión transaccional | regeneración byte a byte |
| F5.4 | caché inmutable concurrente y CLI | digest verificado en cada lectura |
| F5.5 | `DynField` binario/primo y `DynElement` | identidad nominal y encoding estricto |
| F5.6 | `DynBatch` y `DynEngine` | checks de campo/shape una vez por lote |
| F5.7 | puente dinámico → estático | preserva `FieldId` y exige `Proven` |
| F5.8 | adversarial, consumidor externo y SageMath | matriz local completa |

## Dependencias y SOLID

```text
assurance ─────► spec::prime::manifest ─► validation ─► generation
     │                                             ├──► lock/cache
     └─────────► dynamic::field ───────────────────┴──► bridge
                              └──► dynamic::batch
```

- `manifest` solo fija forma y normalización;
- `validation` establece prueba matemática y límites;
- `generation` decide representación y bytes reproducibles;
- lock, filesystem y caché son adaptadores de infraestructura;
- `DynField` posee matemáticas validadas y `DynEngine` solo orquesta lotes;
- el puente reinyecta un manifiesto exportado en la misma factory, sin una
  segunda ruta de confianza.

`num-bigint` solo existe bajo `generator` o `dynamic`. Los tipos estáticos,
`no_std`, kernels mantenidos y API escalar no adquieren esa dependencia.

## Manifiesto primo v1

El documento usa `prime_schema_version = 1` y admite únicamente campos
primos de grado uno, base prima implícita y entero canónico little-endian. Es
deliberadamente distinto del schema binario v1.

Campos normativos:

- `[prime]`: nombre y módulo decimal canónico;
- `[encoding]`: `little`, `canonical` y ancho exacto;
- `[validation]`: `proven` o `probable_prime` con rondas;
- `[certificate]`: Pocklington v1 opcional para `Proven` grande;
- `[build]`: perfil portable, multi-backend o audit.

El descriptor de identidad conserva schema 2, compatible con los campos
primos mantenidos. Nombre, assurance, certificado, perfil y estrategia no
participan en `FieldId`.

## Assurance

`Proven` se establece de dos formas:

1. Miller–Rabin con la base determinista completa para todo `u64`;
2. Pocklington con factores `u64` individualmente probados, reconstrucción de
   `F`, `F * cofactor = p - 1`, `F² > p`, Fermat y gcd por factor.

`ProbablePrime { rounds }` exige entre 16 y 256 rondas en la factory y nunca
autoriza `generate`. En dinámico la etiqueta permanece consultable y se
conserva al exportar el manifiesto.

## Perfiles estáticos

| Rango | Storage | Reducción | Candidato ISA |
|---|---|---|---|
| `p <= 251` | `u8` | Barrett | AVX2 32 lanes |
| `p <= 65_521` | `u16` | Barrett | AVX2 16 lanes |
| `p <= 4_294_967_291` | `u32` | Barrett | AVX2 8 lanes |
| resto | `[u64; N]` | Montgomery radix 64 | BMI2 genérico |

Los adapters externos son siempre explícitos. Compatibilidad estructural no
equivale a una mejora medida para un campo concreto. `PortableOnly` no registra
ISA; `MultiBackend` y `Audit` sí registran candidatos seguros, pero nunca los
promueven a `Auto`.

La salida incluye `mod.rs`, `field.rs`, descriptor, certificado, plan,
vectores, manifiesto normalizado, lock, índice de bundle y README. El runtime
helper ABI sigue siendo 3.

## Lock, publicación y caché

`MicrofieldLock` fija:

- `FieldId`, `ArtifactId`, schema y versión del generador;
- digest del manifiesto canónico;
- perfil, representación y versión de template;
- SHA-256 de cada payload no circular.

La publicación usa staging adyacente y rename del conjunto completo. `check`
regenera en memoria y compara todos los nombres y bytes. La caché usa
`ArtifactId`, es inmutable, relee lock/digests, rechaza symlinks y coordina un
único escritor mediante `create_new`; nunca es autoridad matemática.

## Contextos dinámicos

`DynField` contiene un `Arc` inmutable y soporta:

- campo binario polinómico certificado con Rabin;
- campo primo `u64` probado;
- primo multi-limb probado con Pocklington;
- primo probable explícito para exploración runtime.

`DynElement` no es `Copy`: conserva `FieldId` y `DynLimbStorage`. Hasta ocho
limbs se guardan inline; por encima se usa heap, siempre inicializado y sin
acceso público a limbs. Cada operación escalar rechaza mezcla de campos.

`DynBatch` valida identidad y shape al construirse. `DynEngine` valida los tres
contextos y longitudes una vez por llamada y recorre storage homogéneo sin
repetir `FieldId`. El buffer de salida se reutiliza; la inversión conserva
atomicidad ante cero mediante prefijos antes de publicar resultados.

El nivel honesto actual es `GenericPortable`. No se promete paridad con el
tipo generado ni se reutilizan automáticamente calibraciones estáticas.

## CLI

```text
microfield-gen prime-normalize PRIME.toml [--json]
microfield-gen prime-validate PRIME.toml [--json]
microfield-gen prime-generate PRIME.toml --out ROOT [--json]
microfield-gen prime-check PRIME.toml --out ROOT [--json]
microfield-gen prime-inspect microfield.lock [--json]
```

Los comandos de inspección no escriben. La generación publica un subdirectorio
con el nombre normalizado; no interpreta nombres como paths.

## Gates de cierre

- reproducibilidad de identidades y bytes;
- cambio de perfil altera `ArtifactId`, no `FieldId`;
- pseudoprimos fuertes y certificados alterados se rechazan;
- probable no genera fuente;
- los cuatro perfiles compilan y ejecutan en un crate consumidor;
- lock, índice, check y caché detectan un byte modificado;
- ocho escritores concurrentes publican una única entrada íntegra;
- mezcla escalar/batch y longitudes erróneas preservan salida;
- GF(251) dinámico exhaustivo y GF(2⁸) inversivo exhaustivo;
- frontera inline/heap y límites validados;
- puente binario/primo conserva `FieldId`;
- SageMath 10.7 prueba 16 vectores por cada perfil de aceptación.

Los targets concretos continúan resolviéndose mediante `cfg` y detección segura
del consumidor. No se producen objetos por triple ni ensamblador precompilado:
eso evitaría reproducibilidad multi-target sin aportar corrección a los
adapters target-neutrales ya existentes.
