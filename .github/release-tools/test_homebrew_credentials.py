"""Execute the credential gate without GitHub calls or any real credential."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import yaml


class HomebrewCredentialGate(unittest.TestCase):
    def run_gate(self, token, private='false'):
        root = Path(__file__).resolve().parents[1]
        data = yaml.safe_load((root/'workflows/publish-homebrew.yml').read_text())
        script = data['jobs']['publish']['steps'][0]['run']
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder)/'output'
            env = {**os.environ, 'TAP_TOKEN':token, 'SOURCE_PRIVATE':private,
                   'GITHUB_OUTPUT':str(output)}
            result = subprocess.run(['bash', '-c', script], env=env,
                                    text=True, capture_output=True, check=False)
            return result, output.read_text() if output.exists() else ''

    def test_missing_token_fails_explicitly_instead_of_reporting_no_release(self):
        result, output = self.run_gate('')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('HOMEBREW_TAP_TOKEN is missing', result.stdout)
        self.assertNotIn('No release has been made', result.stdout)
        self.assertNotIn('ready=true', output)

    def test_private_source_does_not_publish(self):
        result, output = self.run_gate('fixture-token', 'true')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('ready=true', output)

    def test_public_configured_source_proceeds_without_echoing_token(self):
        result, output = self.run_gate('fixture-token')
        self.assertEqual(result.returncode, 0)
        self.assertEqual(output, 'ready=true\n')
        self.assertNotIn('fixture-token', result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
