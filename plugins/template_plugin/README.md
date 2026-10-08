# Lyra Plugin Template

Use this boilerplate to create new community plugins for Lyra.

## Structure

- `plugin.py`: Main entrypoint for plugin execution.
- `README.md`: Documentation, dependencies, and configuration.

## Registration

Add your plugin entry to `registry.json` at repository root:

```json
{
  "id": "your-plugin-id",
  "name": "Your Plugin Name",
  "version": "v0.1.0",
  "author": "@your-github",
  "description": "What your plugin does",
  "entry": "plugins/your-plugin-id",
  "status": "active"
}
```
