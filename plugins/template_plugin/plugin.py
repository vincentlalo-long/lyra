"""Template Plugin for Lyra

Example plugin demonstrating standard function interface.
"""

from typing import Any, Dict


def run(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Execute plugin action.

    Args:
        payload: Dictionary containing track/event info from Lyra.

    Returns:
        Result dictionary to return to Lyra.
    """
    return {
        "status": "success",
        "message": "Template plugin executed successfully",
        "data": payload,
    }
