"""Check inward dependency rules without importing or executing application code."""
import json
from pathlib import Path
import re


def violations(root, policy):
    problems = []
    for name in policy.get('absent', []):
        if (root / name).exists():
            problems.append(f'Packaging repository must not contain {name}')
    for layer in policy['layers']:
        matched = 0
        for pattern in layer['paths']:
            for path in sorted(root.glob(pattern)):
                if not path.is_file():
                    continue
                matched += 1
                text = path.read_text()
                name = str(path.relative_to(root))
                for expression in layer.get('forbid', []):
                    if re.search(expression, text, re.M):
                        problems.append(f'{layer["name"]}: forbidden dependency in {name}: {expression}')
                maximum = policy.get('existingSizeExceptions', {}).get(name, 200)
                if len(text.splitlines()) > maximum:
                    problems.append(f'Module exceeds {maximum} lines: {name}')
        if not matched:
            problems.append(f'No source matched required layer: {layer["name"]}')
    return problems


def main():
    root = Path(__file__).resolve().parents[1]
    policy = json.loads((root / '.specify/architecture.json').read_text())
    problems = violations(root, policy)
    if problems:
        print('\n'.join(problems))
    else:
        print('Architecture dependency and module-size checks passed.')
    return int(bool(problems))


if __name__ == '__main__':
    raise SystemExit(main())
