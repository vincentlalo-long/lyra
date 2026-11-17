#!/usr/bin/env python3
import sys
import os

# Add parent directory and optional local py_libs / LYRA_PY_LIBS to sys.path
_root = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, _root)
for _cand in (os.path.join(_root, "py_libs"), os.environ.get("LYRA_PY_LIBS", "")):
    if _cand and os.path.isdir(_cand) and _cand not in sys.path:
        sys.path.insert(0, _cand)

from lyra.cli import main

if __name__ == "__main__":
    main()
