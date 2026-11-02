#!/usr/bin/env python3
"""Reject declared LRS expansion outputs in the Git index, including forced adds."""
import json
from pathlib import Path
import subprocess

INVENTORY = 'spec/conformance/lrs-generated-artifacts.json'


def tracked_outputs(root):
    """Inspect the exact staged recipe rather than a possibly unstaged edit."""
    recipe = json.loads(subprocess.check_output(['git', 'show', ':' + INVENTORY], cwd=root))
    outputs = {name for stage in recipe['stages'] for name in stage['outputs']}
    tracked = set(subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0'))
    return sorted(outputs & tracked)


if __name__ == '__main__':
    found = tracked_outputs(Path.cwd())
    if found:
        raise SystemExit('Generated outputs must stay untracked:\n' + '\n'.join(found))
    print('No declared generated outputs in the Git index')
