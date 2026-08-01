# ADR 0010 — Factory binaria estática y ABI de codegen

**Estado:** aceptado.

## Contexto

Los tres campos mantenidos demostraron la API y los algoritmos portables, pero
un consumidor no podía introducir otro GF(2^m) sin modificar Microfield. Una
factory runtime obligaría a guardar grado, módulo y contexto en cada operación
o a usar dispatch dinámico, degradando layout, optimización y seguridad de
tipos.

## Decisión

La factory es un generador previo a compilación. Manifiesto y Builder producen
el mismo `FieldManifest`; después se ejecutan normalización, `FieldId`, Rabin,
planificación y render determinista. El resultado es un newtype nominal con
tamaños literales, limbs privados y llamadas estáticas a helpers portables.

El módulo `microfield::__private` es público y oculto de la documentación
normal solo porque el código generado vive en otro crate. No es una API para
programación manual. Queda congelado el ABI de codegen 1:

- suma XOR, prueba de cero y encoding de arrays de `u64`;
- producto carry-less y cuadrado con producto ancho literal;
- reducción por el módulo completo;
- `mul_by_x` y reducción de bytes polinómicos;
- inversión, Frobenius y traza genéricos;
- construcción inmutable de `StaticFieldSpec`.

Cada fuente contiene `assert!(supports_codegen_abi(1))` en contexto `const`.
Una fuente incompatible falla al compilar. Al introducir ABI N, el runtime
conservará los símbolos de N-1 durante al menos un ciclo menor y el rango
`MIN_CODEGEN_ABI_VERSION..=MAX_CODEGEN_ABI_VERSION` lo expresará. Los símbolos
existentes no cambiarán de semántica; una firma incompatible recibirá nombre o
namespace nuevo.

El package digest usa el dominio
`microfield:generated-field-package:v1\0` y cubre el digest del bundle, la
longitud y todos los bytes de la fuente Rust. Por tanto, un cambio de codegen
no puede conservar silenciosamente la huella de publicación.

## Consecuencias

- El elemento no guarda módulo, identidad, heap, backend ni flags.
- Dos definiciones generan tipos incompatibles nominalmente.
- La aritmética escalar queda monomorfizada; batch conserva una indirección por
  lote.
- El generador requiere `std`, pero la fuente y el runtime portable son
  `no_std`.
- Los helpers generales priorizan corrección y cobertura de grados. Los presets
  128/256 conservan especializaciones mientras el ensamblado/benchmark las
  justifique.
- La elegibilidad ISA no puede declararla libremente un crate externo; se
  resolverá mediante perfiles internos certificados en H2.2.

## Alternativas rechazadas

- `Box<dyn Field>` o un contexto runtime: pierde tipo nominal y dispatch
  estático.
- const generics con expresiones inestables: rompe MSRV y no aporta mejor
  diagnóstico.
- macro que acepta tokens arbitrarios: debilita normalización, certificados y
  reproducibilidad.
- copiar toda la matemática en cada archivo: multiplica superficie de auditoría
  y permite divergencias entre campos.
