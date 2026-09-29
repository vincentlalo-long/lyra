#!/usr/bin/env python3
import sys
import os

# Add parent directory to sys.path so 'lyra' package can be imported
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from lyra.cli import main

if __name__ == "__main__":
    main()
