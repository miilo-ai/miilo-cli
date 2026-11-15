"""Exercise formula trust, candidate caching and failure propagation without installing tools."""
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'release-tools/check-homebrew-formula.sh'
BREW = '''#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$FIXTURE/log"
case "$1" in
  tap) ;;
  trust) [ "${TRUST_FAIL:-0}" = 0 ]; touch "$FIXTURE/trusted" ;;
  audit) test -f "$FIXTURE/trusted" ;;
  info) printf '{"formulae":[{"urls":{"stable":{"checksum":"%s"}}}]}\\n' "$EXPECTED_SHA" ;;
  --cache) printf '%s\\n' "$FIXTURE/cache/candidate.tar.xz" ;;
  install) [ "${INSTALL_FAIL:-0}" = 0 ]; test -f "$FIXTURE/trusted" ;;
  test|uninstall) ;;
  --prefix) printf '%s\\n' "$FIXTURE/prefix" ;;
  *) exit 9 ;;
esac
'''


class HomebrewRehearsalTests(unittest.TestCase):
    def exercise(self, **overrides):
        with tempfile.TemporaryDirectory() as folder:
            stage = Path(folder)
            (stage / 'prefix/bin').mkdir(parents=True)
            (stage / 'tools').mkdir()
            brew = stage / 'brew'
            brew.write_text(BREW)
            brew.chmod(0o755)
            local = stage / 'prefix/bin/miilo-local'
            local.write_text('#!/bin/sh\nexit 0\n')
            local.chmod(0o755)
            (stage / 'tools/test-archive-install.sh').write_text('test -n "$1"\n')
            archive = stage / 'archive'
            archive.write_bytes(b'candidate archive')
            env = dict(os.environ, EXPECTED_SHA=hashlib.sha256(archive.read_bytes()).hexdigest(), FIXTURE=folder, PATH=folder + ':' + os.environ['PATH'])
            env.update(overrides)
            result = subprocess.run(['bash', str(SCRIPT), folder, str(archive)], env=env,
                                    text=True, capture_output=True, timeout=10)
            cache = stage / 'cache/candidate.tar.xz'
            return result, (stage / 'log').read_text().splitlines(), cache.read_bytes() if cache.exists() else None

    def test_specific_trust_precedes_audit_and_candidate_is_installed(self):
        result, calls, cached = self.exercise()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls[1:3], ['trust --formula miilo-ai/cli/miilo', 'audit --strict miilo-ai/cli/miilo'])
        self.assertEqual(cached, b'candidate archive')
        self.assertIn('test miilo-ai/cli/miilo', calls)
        self.assertEqual(calls[-1], 'uninstall miilo-ai/cli/miilo')

    def test_trust_failure_stops_before_audit_or_install(self):
        result, calls, cached = self.exercise(TRUST_FAIL='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 2)
        self.assertIsNone(cached)

    def test_wrong_candidate_checksum_stops_before_install(self):
        result, calls, cached = self.exercise(EXPECTED_SHA='0' * 64)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('install miilo-ai/cli/miilo', calls)
        self.assertIsNone(cached)

    def test_install_failure_is_not_hidden(self):
        result, calls, _ = self.exercise(INSTALL_FAIL='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls[-1], 'install miilo-ai/cli/miilo')

    def test_shared_checker_is_required_by_rehearsal_and_publication(self):
        def workflow(name):
            return yaml.safe_load((ROOT / 'workflows' / name).read_text())
        builds = workflow('build-release-artifacts.yml')['jobs']
        self.assertIn('homebrew-rehearsal', builds['ready']['needs'])
        rehearsal = builds['homebrew-rehearsal']
        self.assertNotIn('environment', rehearsal)
        self.assertNotIn('secrets.', str(rehearsal))
        for steps in [rehearsal['steps'], workflow('publish-homebrew.yml')['jobs']['publish']['steps']]:
            self.assertIn('check-homebrew-formula.sh', str(steps))
        self.assertNotIn('HOMEBREW_NO_REQUIRE_TAP_TRUST', SCRIPT.read_text())


if __name__ == '__main__':
    unittest.main()
