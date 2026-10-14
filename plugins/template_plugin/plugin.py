"""Template Plugin for Lyra

Example plugin demonstrating the standard interface.

Contract (required for the TUI Store runner):
- `run(payload: dict) -> dict` with `{"status", "message", "data"}`.
- Answer `{"action": "list"}` with a capability description so the Store
  `x` (Run) key shows something useful without side effects.
"""

from typing import Any, Dict


PLUGIN_ID = "template-plugin"
PLUGIN_VERSION = "v0.1.0"


def run(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Execute plugin action.

    Args:
        payload: Dictionary containing track/event info from Lyra.
            `{"action": "list"}` returns the capability descriptor.

    Returns:
        Result dictionary to return to Lyra.
    """
    action = (payload or {}).get("action", "run")
    if action == "list":
        return {
            "status": "success",
            "message": "template-plugin v0.1.0: boilerplate example (echoes payloads)",
            "data": {"id": PLUGIN_ID, "version": PLUGIN_VERSION, "actions": ["list", "run"]},
        }
    return {
        "status": "success",
        "message": "Template plugin executed successfully",
        "data": payload,
    }
