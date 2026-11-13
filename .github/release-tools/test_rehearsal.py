"""Release rehearsal must build real payloads without publication authority."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import yaml

ROOT = Path(__file__).resolve().parents[1] / 'workflows'


def workflow(name):
    return yaml.safe_load((ROOT / name).read_text())


class RehearsalTests(unittest.TestCase):
    def test_build_workflows_have_no_publication_authority(self):
        for name in ['build-release-artifacts.yml', 'build-node-payload.yml']:
            data = workflow(name)
            self.assertEqual(data['permissions'], {'contents': 'read'})
            for job in data['jobs'].values():
                self.assertEqual(job.get('permissions', data['permissions']), {'contents': 'read'})
                for step in job.get('steps', []):
                    self.assertNotIn('dist host', step.get('run', ''))
                    self.assertNotIn('gh release create', step.get('run', ''))
                    self.assertNotIn('actions/attest', step.get('uses', ''))

    def test_qualification_cannot_pass_a_failed_or_skipped_rehearsal(self):
        ci = workflow('ci.yml')['jobs']
        self.assertEqual(ci['qualification']['needs'], 'release-rehearsal')
        self.assertEqual(ci['qualification']['if'], 'always()')
        self.assertIn("result == 'success'", ci['qualification']['with']['artifacts-ready'])
        first = workflow('qualify.yml')['jobs']['qualify']['steps'][0]
        self.assertEqual(first['if'], 'inputs.artifacts-ready != true')
        self.assertIn('exit 1', first['run'])

    def test_tagging_and_publication_use_the_same_build_as_prs(self):
        ci = workflow('ci.yml')['jobs']['release-rehearsal']['uses']
        version = workflow('version.yml')['jobs']
        self.assertEqual(version['rehearse']['uses'], ci)
        self.assertIn('rehearse', version['qualify']['needs'])
        self.assertIn('needs.rehearse.result', version['qualify']['with']['artifacts-ready'])
        release = workflow('release.yml')['jobs']
        self.assertEqual(release['plan']['uses'], ci)
        self.assertEqual(release['host']['environment'], 'release')

    def test_readiness_requires_every_build_and_writes_a_real_output(self):
        ready = workflow('build-release-artifacts.yml')['jobs']['ready']
        for failed_result in [None, 'failure', 'skipped', 'cancelled']:
            results = {name: {'result': 'success'} for name in ready['needs']}
            if failed_result:
                results['build-global-artifacts']['result'] = failed_result
            with tempfile.TemporaryDirectory() as folder:
                output = Path(folder) / 'output'
                env = dict(os.environ, RESULTS=json.dumps(results), GITHUB_OUTPUT=str(output))
                result = subprocess.run(['bash', '-e', '-c', ready['steps'][0]['run']], env=env,
                                        capture_output=True, text=True, timeout=10)
                if failed_result:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse(output.exists())
                else:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(output.read_bytes(), b'ready=true\n')


if __name__ == '__main__':
    unittest.main()
