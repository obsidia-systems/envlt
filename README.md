# envlt

`envlt` es una herramienta CLI local-first escrita en Rust para guardar variables de entorno en un vault cifrado y regenerar `.env` bajo demanda.

Estado actual: MVP de Fase 1 en progreso.

## Qué ya funciona

- vault local cifrado con `age`
- importación desde `.env`
- listado de proyectos
- actualización de variables
- generación de `.env`
- ejecución de procesos con variables inyectadas
- creación y lectura de `.envlt-link`
- backup básico automático del vault
- exportación e importación de bundles `.evlt`

## Uso rápido

```bash
cargo run -p envlt-cli -- init
cargo run -p envlt-cli -- add api-payments
cargo run -p envlt-cli -- list
cargo run -p envlt-cli -- set --project api-payments PORT=4000
cargo run -p envlt-cli -- use --project api-payments
cargo run -p envlt-cli -- run --project api-payments -- node server.js
cargo run -p envlt-cli -- export api-payments --out bundle.evlt
cargo run -p envlt-cli -- import bundle.evlt
cargo run -p envlt-cli -- import bundle.evlt --overwrite
```

Si ya existe `.envlt-link` en el directorio actual, `set`, `use` y `run` también pueden resolver el proyecto sin `--project`.

## Documentación

- [Uso del MVP actual](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/usage.md)
- [Estado del proyecto](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/status.md)
- [Plan de implementación Fase 1](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/phase-1-implementation-plan.md)
- [Plan siguiente de implementación](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/next-implementation-plan.md)
- [Definición del proyecto](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/project-definition.md)

## Calidad

Validado con:

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
