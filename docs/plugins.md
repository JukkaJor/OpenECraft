# Plugins

OpenECraft is extended through the `openecraft-api` crate. A plugin adds

| Extension | What it is | Editor | Command line / MCP |
| --- | --- | --- | --- |
| Command | function on the project with typed parameters, returns JSON | *Plugins* menu (form for the parameters) | a command / tool of its own |
| Check | adds issues to the project check | *Check* tab | `check` |
| Report | table generated from the project | *Plugins* menu | `report --kind <name>` |
| Exporter | text file generated from the project | *Plugins* menu | `export --format <name>` |

Names are snake_case and unique among all plugins; commands must not take the name of
a built-in tool. Changes a command makes to the project are one undo step.
`openecraft-cli plugins` lists the loaded plugins, *Plugins › Manage plugins…* in the
editor enables and disables them (remembered by the editor).

## Rust plugins

Built-in plugins (project statistics, panel layout checks, panel manufacturing, part
lifecycle) live in
`crates/openecraft-api/src/builtin.rs` and its submodules and are compiled into
every front end (also the web version). A plugin implements `Plugin`:

```rust
use openecraft_api::{param, Info, Kind, Plugin, Registry, Table, Text};
use openecraft_core::reports::Issue;
use openecraft_i18n::tr_const;

pub struct Spares;

impl Plugin for Spares {
    fn info(&self) -> Info {
        Info { id: "spares".into(), name: Text::Borrowed(tr_const!("Spare parts")), version: "1.0".into(), description: Text::Borrowed("") }
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        reg.command("set_spares", tr_const!("Sets the spare quantity of a part"), vec![param("number", Kind::Str, tr_const!("Part number"))], |prj, args| {
            // change `prj`, return JSON
            Ok(serde_json::json!({ "part": args.req("number") }))
        })
        .check("spares_missing", tr_const!("Spare parts"), |prj, analysis| Ok(Vec::<Issue>::new()))
        .report("spares", tr_const!("Spare parts"), |prj, analysis| Ok(Table::new([tr_const!("Part")])))
        .exporter("spares_csv", tr_const!("Spare parts (CSV)"), "csv", |prj| Ok(String::new()));
        Ok(())
    }
}
```

Texts given as `&'static str` are English catalog entries (`tr_const!`, see AGENTS.md)
and are shown translated.

## External plugins

An external plugin is a program in any language in a folder of its own:

```
my_plugin/
  plugin.json
  main.py
```

```json
{
  "id": "my_plugin",
  "name": "My plugin",
  "version": "1.0",
  "description": "What it does",
  "api": 1,
  "run": ["python3", "main.py"]
}
```

The folders containing plugin folders are given in the environment variable
`OPENECRAFT_PLUGINS` (separated like `PATH`) for the editor and the command line, or
with `openecraft-cli --plugins <folder>`. External plugins need processes and are not
available in the browser.

### Protocol

For every request OpenECraft starts the program (`run`, in the plugin folder), writes
one JSON object to its stdin and reads one JSON object from its stdout. Every request
has `"api": 1` and a `method`. An answer `{"error": "message"}` (or a non-zero exit
code, with the message on stderr) is an error. `project` is the project as in a
`.oec` file.

| Request | Answer |
| --- | --- |
| `{"method": "describe"}` | `{"commands": [{"name", "help", "params": [{"name", "kind", "required", "help"}]}], "checks": [{"name", "title"}], "reports": [{"name", "title"}], "exporters": [{"name", "title", "extension"}]}` (all lists optional) |
| `{"method": "command", "name", "params": {…}, "project": {…}}` | `{"result": <any JSON>, "project": {…}}` – `project` only if the command changed it |
| `{"method": "check", "name", "project"}` | `{"issues": [{"severity": "error" \| "warning" \| "info", "message", "page": <page index>, "item": <item id>}]}` – `page` / `item` optional |
| `{"method": "report", "name", "project"}` | `{"headers": ["…"], "rows": [["…"]]}` |
| `{"method": "export", "name", "project"}` | `{"content": "file content"}` |

Parameter kinds: `str`, `int`, `num`, `bool`, `ids` (list of item ids), `points`
(list of `[x, y]` in mm), `object` (JSON object). `required` defaults to `true`.

`describe` is asked once when the plugin is loaded; checks run after every change of
the project in the editor, so they should answer quickly.

A complete example is in [`examples/plugins/page_notes`](../examples/plugins/page_notes).
