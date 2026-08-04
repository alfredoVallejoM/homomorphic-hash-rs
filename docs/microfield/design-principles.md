# SOLID aplicado a Rust

## Responsabilidad única

Una unidad tiene un motivo de cambio:

- `field`: significado algebraico;
- `binary`: algoritmos de GF(2^n);
- `kernel`: contrato de ejecución batch;
- `engine`: selección y fachada;
- `backend`: ejecución concreta;
- `spec`: descripción, validación y generación.

Dentro de `spec`, `PortableOptimizer` solo clasifica planes; no valida
polinomios, renderiza Rust, ejecuta benchmarks ni publica archivos.

No se crearán módulos `utils` genéricos. Una utilidad pertenece al dominio que
define sus invariantes.

## Abierto/cerrado

Los algoritmos operan sobre traits internos y capacidades públicas. Añadir un
campo genera un nuevo tipo, metadatos y catálogo; no introduce ramas por
`FieldId`. Añadir un backend registra otra estrategia sin alterar el elemento.
Las nuevas reducciones se añaden como variantes cerradas del IR versionado y
helpers monomorfizados, sin editar los consumidores algebraicos.

## Sustitución de Liskov

Un tipo solo se declara `Field` cuando supera la misma suite genérica de leyes.
Los tests comprueban identidades, asociatividad, distributividad, inversa y
roundtrip canónico. Un backend solo es sustituible si coincide bit a bit con el
portable.

## Segregación de interfaces

`Field` contiene únicamente operaciones algebraicas básicas. Cuadrado,
inversión, potencia, encoding, extensión y metadatos son capacidades
independientes. Los bounds genéricos deben pedir el trait mínimo necesario.

## Inversión de dependencias

Las políticas de alto nivel dependen de contratos:

- `Engine` depende de `KernelSet`;
- los casos de uso del generador dependen de ports;
- la validación depende de descriptores normalizados;
- los backends consumen planes matemáticos, no los crean.

## Criterio de coste cero

En el hot path se aceptan únicamente abstracciones monomorfizadas, constantes
generadas o una selección previa por lote. Se rechazan trait objects, heap,
estado global mutable y detección de CPU por elemento.
