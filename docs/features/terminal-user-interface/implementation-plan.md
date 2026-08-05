# Plan de implementación: Terminal User Interface (TUI)

## Objetivo

Ofrecer una interfaz interactiva para descubrir y operar un Vault local sin
duplicar las reglas de dominio ni relajar el tratamiento de Secrets. La TUI es
un adaptador sobre `AppService`; el Vault cifrado sigue siendo la única fuente
de verdad y `envlt run` continúa siendo preferible a materializar un `.env`.

## Decisiones de arquitectura

| Decisión | Resultado |
| --- | --- |
| Ubicación | Añadir `crates/envlt-tui` al workspace existente. No crear otro repositorio. |
| Límites | `envlt-core` no depende de la TUI. `envlt-tui` depende de `envlt-core`; `envlt-cli` conserva Clap, autenticación y despacho. |
| Entrada inicial | `envlt` sin subcomando abre la TUI; `envlt tui` sigue siendo una entrada explícita. Con Project link abre su contexto y, sin link, muestra los Projects del Vault. |
| Motor | `ratatui` con backend `crossterm`; ciclo síncrono de eventos. No introducir Tokio en el MVP. |
| Operaciones | La TUI llama a `AppService` para leer y mutar. Nunca abre, descifra ni escribe `vault.age` por su cuenta. |
| Estado | El crate mantiene sólo estado de presentación: pantalla, foco, selección, filtro, modal y notificación. El estado de dominio se recarga desde `AppService` después de cada mutación. |

La interfaz recibirá desde `envlt-cli` un `AppService`, el directorio actual y
la passphrase ya resuelta. Esto conserva la precedencia actual de autenticación
y evita que la TUI implemente una ruta paralela de keyring o `ENVLT_PASSPHRASE`.

## Forma del crate

```text
crates/envlt-tui/src/
  lib.rs        # run(service, context) y API pública mínima
  app.rs        # App, Screen, Focus, Modal y transiciones puras
  event.rs      # conversión de eventos Crossterm a Action
  action.rs     # acciones semánticas, sin teclas incrustadas en widgets
  view.rs       # renderizado Ratatui por pantalla
  form.rs       # edición de texto, validación local y máscaras
  presenter.rs  # adaptador AppService -> modelos seguros de pantalla
  terminal.rs   # ciclo de vida de ratatui::run y restauración de terminal
```

`envlt-cli` añade un adaptador delgado para `Commands::Tui`; no traslada lógica
de Vault ni de Project a comandos nuevos. El `App` no almacenará valores de
Secrets en logs, errores, `Debug` o notificaciones.

## Alcance por fases

### Fase 0 — contrato y base técnica

1. Añadir `envlt-tui` al workspace y las dependencias `ratatui` y `crossterm`.
2. Implementar entrada/salida segura de pantalla alternativa, raw mode y pánico;
   toda salida debe restaurar la terminal.
3. Definir `Action`, `Screen`, `Modal` y un `App` reducible y testeable sin TTY.
4. Añadir `envlt tui` y la entrada por defecto `envlt`; sin Project link, abrir
   una lista de Projects en vez de fallar por falta de contexto.

**Criterio de salida:** `envlt tui` abre y cierra limpiamente; `Esc` y `Ctrl-C`
no dejan el terminal en raw mode; CI ejecuta pruebas de estado sin TTY.

### Fase 1 — exploración segura del Vault (MVP)

1. Resolver Project y Environment con las mismas reglas que la CLI.
2. Pantalla principal: Vault, Project, Environment, estado de Project link y
   conteos de variables por `VarType`.
3. Lista navegable y filtrable de variables usando
   `AppService::project_variable_views`.
4. Mostrar nombre, tipo y fecha; las Variables `Secret` muestran máscara fija,
   nunca longitud ni valor.
5. Selector de Project y Environment; pantalla Doctor basada en
   `AppService::doctor`.
6. Ayuda de teclado y estados de carga/error vacíos.

**Criterio de salida:** el usuario puede inspeccionar un Vault, navegar entre
Projects y Environments, filtrar Variables y ejecutar Doctor sin que un valor
secreto llegue a pantalla, stdout o stderr.

### Fase 2 — mutaciones controladas

1. Formulario para crear y editar Variables usando `AppService::set_variable`.
2. Selector explícito de `VarType`; la inferencia existente se conserva cuando
   no se elige tipo.
3. Borrado lógico mediante `AppService::unset_variable`, con modal que nombra
   la Variable y exige confirmación inequívoca.
4. Tras éxito, recargar la vista desde el servicio y mostrar sólo mensajes sin
   valores: por ejemplo, `Variable DATABASE_URL actualizada`.
5. Incluir historial de una Variable si el modelo de vista actual lo expone de
   manera segura.

**Criterio de salida:** altas, cambios de tipo y borrados producen exactamente
el mismo estado persistido que los comandos CLI equivalentes, cubierto con
tests de `envlt-core` y de flujo de TUI.

### Fase 3 — operaciones con efecto externo

1. Añadir una vista previa segura de `pull`; advertir que materializa un `.env`
   en plaintext y pedir confirmación antes de escribir.
2. Añadir exportación/importación de Bundle como flujos separados, con preview,
   destino explícito y confirmación de sobreescritura.
3. Considerar `envlt run` desde la TUI sólo cuando el modelo de restauración de
   terminal y el manejo del proceso hijo estén definidos y probados.

**Criterio de salida:** ninguna operación escribe un `.env` ni un Bundle sin
una confirmación visible; cancelar deja el Vault y el filesystem intactos.

### Fase 4 — mejoras posteriores al MVP

- Revelado temporal de un Secret, opt-in y con ocultado automático al cambiar
  foco/pantalla o tras un timeout. No clipboard en la primera iteración.
- Búsqueda avanzada, diff contra `.env.example`, generación de valores y
  accesibilidad/compatibilidad con terminales pequeños.

## Reglas de seguridad no negociables

- Las vistas, errores, toasts, panics y tests no contienen valores de `Secret`.
- El campo de edición de Secret no se vuelve a poblar con el valor existente;
  cambiarlo requiere introducir uno nuevo conscientemente.
- No hay copia a clipboard, log de acciones ni persistencia de estado de UI en
  el MVP.
- Toda acción que materializa plaintext, revela un Secret o modifica/elimina
  datos usa una confirmación específica y cancelable.
- La TUI no ejecuta shell a través de `sh -c`; cualquier futuro `run` conserva
  el modelo de argumentos de la CLI.

## Estrategia de pruebas y calidad

| Nivel | Cobertura |
| --- | --- |
| `envlt-tui` unitarias | Reducer de acciones, foco, filtros, modales, máscaras y mensajes seguros. |
| `envlt-tui` render | `TestBackend` de Ratatui para layout mínimo, estados vacíos y tamaños pequeños. |
| Integración | Vault temporal con valores falsos; verificar que render y errores no contienen `example-secret`. |
| CLI | `envlt tui` maneja Vault inexistente, Project link inválido y salida limpia. |
| Regresión | `make check`, además de pruebas explícitas de restauración de terminal cuando sea viable en CI. |

## Orden de entrega recomendado

1. PR 1: workspace, crate vacío, `envlt tui`, lifecycle de terminal y tests.
2. PR 2: modelo de estado, encabezado y lista segura de Variables.
3. PR 3: navegación Project/Environment, filtro y Doctor.
4. PR 4: crear/editar/eliminar Variables con confirmaciones.
5. PR 5: `pull` y Bundles, si los flujos de confirmación ya están maduros.

Cada PR debe ser funcional por sí mismo, mantener `make check` verde y evitar
cambios al formato de `VaultData` o Bundle.
