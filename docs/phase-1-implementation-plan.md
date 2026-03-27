# Plan de implementación — Fase 1

## Contexto

Este documento aterriza la **Fase 1 — MVP funcional** definida en [project-definition.md](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/project-definition.md). El objetivo es entregar una primera versión usable para desarrollo local con:

- vault local cifrado
- comandos básicos operativos
- parser de `.env`
- tests de integración para flujos críticos

## Objetivo de la fase

Construir un MVP de `envlt` que permita a una persona:

1. inicializar un vault local cifrado
2. importar variables desde un `.env`
3. listar proyectos guardados
4. actualizar variables de un proyecto
5. generar un `.env` desde el vault
6. ejecutar un proceso con variables inyectadas sin persistir secretos en disco

## Alcance exacto

Incluye:

- workspace Cargo mínimo con `envlt-core` y `envlt-cli`
- almacenamiento local de vault cifrado con `age`
- comandos `init`, `add`, `list`, `set`, `use`, `run`
- parser y writer de `.env`
- tests unitarios y de integración del flujo base

No incluye en esta fase:

- export/import `.evlt`
- sync con nube
- Keychain
- GUI
- diff, gen, doctor, auth, sync

## Principios técnicos

- `envlt-core` contiene toda la lógica de dominio; el CLI solo orquesta I/O.
- El vault se modifica siempre mediante una API de aplicación, nunca desde comandos sueltos.
- Las operaciones de escritura son atómicas: escribir a archivo temporal y luego `rename`.
- Los secretos no se imprimen en stdout ni se incluyen en mensajes de error.
- Tipos de error explícitos con `thiserror`; `anyhow` solo en el binario.
- Tests aislados con `tempfile` y rutas temporales por caso.
- Diseño preparado para crecer a Fase 2+ sin reestructuras costosas.

## Recomendaciones 2026 para Rust aplicadas a esta fase

- Mantener arquitectura `workspace-first`: `core` desacoplado del binario desde el día uno.
- Preferir APIs pequeñas y tipadas sobre helpers globales o estado implícito.
- Separar claramente dominio, persistencia, crypto y capa CLI.
- Modelar secretos con cuidado desde el inicio. Recomendado: evaluar `secrecy` para reducir exposición accidental en `Debug` y logs.
- Usar `PathBuf` solo en bordes de sistema; dentro del dominio, mantener tipos semánticos simples.
- Hacer pruebas de integración de CLI desde el inicio para proteger UX y regresiones.
- Diseñar para compatibilidad con Rust 2024 en estilo, aunque el PDD hoy marque Rust 2021.

## Estructura propuesta para Fase 1

```text
envlt/
├── Cargo.toml
├── rust-toolchain.toml
├── .gitignore
├── crates/
│   ├── envlt-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app/
│   │       │   ├── mod.rs
│   │       │   └── service.rs
│   │       ├── env/
│   │       │   ├── mod.rs
│   │       │   ├── parser.rs
│   │       │   └── writer.rs
│   │       ├── vault/
│   │       │   ├── mod.rs
│   │       │   ├── crypto.rs
│   │       │   ├── model.rs
│   │       │   └── store.rs
│   │       └── error.rs
│   └── envlt-cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── cli.rs
│           └── commands/
│               ├── init.rs
│               ├── add.rs
│               ├── list.rs
│               ├── run.rs
│               ├── set.rs
│               └── use_cmd.rs
└── tests/
    └── cli/
```

## Decisiones de arquitectura

### 1. Workspace mínimo

Arrancar solo con `envlt-core` y `envlt-cli`. `envlt-bar` queda fuera del scaffold inicial para reducir ruido y acelerar entrega del MVP.

### 2. Capa de aplicación en `envlt-core`

Además de `vault/` y `env/`, conviene tener una capa `app/service.rs` con casos de uso:

- `init_vault`
- `add_project_from_env_file`
- `list_projects`
- `set_variable`
- `materialize_env_file`
- `run_with_project_env`

Esto evita meter lógica de negocio dentro de `clap` y facilita reutilización futura por GUI o Tauri.

### 3. Vault versionado desde el primer commit

`VaultData.version` debe existir desde el inicio y validarse al cargar. Aunque solo exista una versión, esto evita migraciones dolorosas en Fase 2+.

### 4. Escrituras atómicas y backup básico

En Fase 1 no hace falta un sistema completo de backups, pero sí:

- escribir a un temporal en el mismo directorio
- `fsync` cuando aplique
- reemplazo atómico por `rename`

Si hay tiempo, agregar `vault.age.bak` antes de sobreescrituras.

### 5. Resolución explícita del proyecto

Para Fase 1, `add`, `set`, `use` y `run` deben aceptar `--project`. Se puede permitir inferir desde `.envlt-link` más adelante. Esto reduce magia y simplifica el MVP.

## Modelo de datos mínimo

Para esta fase, el modelo puede empezar así:

```rust
pub struct VaultData {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub projects: BTreeMap<String, Project>,
}

pub struct Project {
    pub name: String,
    pub path: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub variables: BTreeMap<String, Variable>,
}

pub struct Variable {
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Notas:

- `BTreeMap` da orden estable para snapshots, tests y diffs futuros.
- `VarType` puede diferirse a Fase 3 para mantener el MVP pequeño.
- `description` y `generated` no son necesarias para desbloquear Fase 1.

## Dependencias sugeridas para Fase 1

En línea con el PDD, pero reducidas al mínimo útil:

- `clap` con `derive`
- `serde`
- `toml`
- `age`
- `thiserror`
- `anyhow`
- `chrono` con `serde`
- `dirs`
- `tempfile`
- `assert_cmd`
- `predicates`

Opcionales pero recomendadas:

- `secrecy`
- `zeroize`

## Plan por entregables

### Entregable 1. Scaffold del workspace

Resultado esperado:

- workspace compila
- `cargo fmt`, `cargo clippy`, `cargo test` existen desde el inicio
- crates `envlt-core` y `envlt-cli` conectados

Tareas:

- crear `Cargo.toml` de workspace
- fijar MSRV en `rust-toolchain.toml`
- configurar perfiles de compilación razonables
- añadir `.gitignore` para `.env`, `target/`, archivos temporales y vault local de pruebas

### Entregable 2. Dominio y errores

Resultado esperado:

- modelos base del vault definidos
- errores tipados y mensajes de usuario claros

Tareas:

- implementar `VaultData`, `Project`, `Variable`
- crear `EnvltError`
- definir aliases de `Result`
- agregar constructores y helpers mínimos

### Entregable 3. Persistencia y crypto

Resultado esperado:

- crear, leer, descifrar y sobrescribir `~/.envlt/vault.age`

Tareas:

- resolver home dir y ruta del vault
- serializar `VaultData` a TOML
- cifrar con `age` usando passphrase
- descifrar y validar versión
- persistencia atómica

Criterios técnicos:

- si el vault no existe, errores claros
- si la passphrase falla, error específico
- nunca dejar un vault parcialmente escrito

### Entregable 4. Parser y writer de `.env`

Resultado esperado:

- importar y exportar variables simples de forma estable

Tareas:

- soportar líneas vacías y comentarios
- parsear `KEY=VALUE`
- preservar valores tal cual cuando no haga falta normalización
- escribir `.env` ordenado por clave para reproducibilidad

Fuera de alcance en Fase 1:

- interpolación compleja
- expansión de variables
- compatibilidad exacta con todos los edge cases de shells

### Entregable 5. Capa de aplicación

Resultado esperado:

- casos de uso independientes del CLI

Tareas:

- `init_vault(passphrase)`
- `add_project_from_env_file(project, path, passphrase)`
- `list_projects(passphrase)`
- `set_variable(project, key, value, passphrase)`
- `write_env_file(project, output_path, passphrase)`
- `build_process_env(project, passphrase)`

### Entregable 6. CLI usable

Resultado esperado:

- experiencia mínima coherente y documentada

Comandos a entregar:

- `envlt init`
- `envlt add <project> [--file <path>]`
- `envlt list`
- `envlt set --project <project> <KEY=VALUE>`
- `envlt use --project <project> [--out .env]`
- `envlt run --project <project> -- <command> [args...]`

Decisiones UX:

- usar prompts de passphrase simples para `init` y acceso al vault
- exigir `--` en `run` para separar flags de `envlt` y del proceso hijo
- output corto, claro y estable para tests

### Entregable 7. Tests de integración

Resultado esperado:

- el flujo principal queda protegido por tests automatizados

Casos mínimos:

1. `init` crea un vault válido
2. `add` ingiere un `.env` y guarda un proyecto
3. `list` muestra el proyecto agregado
4. `set` actualiza una variable existente o crea una nueva
5. `use` materializa un `.env` con el contenido esperado
6. `run` inyecta variables a un proceso hijo
7. passphrase incorrecta devuelve error usable

## Orden recomendado de implementación

1. scaffold del workspace
2. modelos y errores
3. store en claro con tests locales
4. cifrado `age`
5. parser/writer `.env`
6. capa de aplicación
7. CLI `init`, `list`
8. CLI `add`, `set`
9. CLI `use`
10. CLI `run`
11. tests de integración y endurecimiento

Este orden reduce el riesgo porque primero estabiliza el dominio y persistencia antes de sumar UX y procesos hijos.

## Riesgos de la fase y mitigación

### Riesgo: complejidad del manejo de passphrase

Mitigación:

- empezar con prompt simple y sin Keychain
- no intentar caching de passphrase en memoria en Fase 1

### Riesgo: parser `.env` demasiado ambicioso

Mitigación:

- documentar un subconjunto soportado
- cubrirlo bien con tests
- expandir compatibilidad en Fase 3 si hace falta

### Riesgo: `run` multiplataforma

Mitigación:

- usar `std::process::Command`
- inyectar variables por proceso, no por shell
- testear con comandos mínimos y portables

### Riesgo: cambios de estructura en Fase 2

Mitigación:

- mantener `envlt-core` con capas separadas desde el inicio
- versionar el vault desde el primer formato

## Definition of Done

La Fase 1 se considera terminada cuando:

- `cargo test` pasa en local
- `cargo clippy --all-targets --all-features -D warnings` pasa
- `envlt init` crea un vault cifrado funcional
- `envlt add`, `list`, `set`, `use` y `run` funcionan de extremo a extremo
- el flujo completo no requiere editar manualmente el vault
- los errores principales son entendibles para una persona usuaria
- existe documentación corta de uso en `README.md`

## Backlog inmediato al cerrar la fase

- soporte de `.envlt-link`
- ocultado parcial de secretos en salidas
- backup automático explícito
- import/export `.evlt`
- inferencia de `VarType`
- `doctor` para diagnóstico

## Recomendación final

Para esta Fase 1 conviene optimizar por **simplicidad operativa** y no por completitud. Si una decisión mejora arquitectura futura pero complica demasiado el MVP, la prioridad debe ser entregar un flujo robusto `init -> add -> set -> use/run`.
