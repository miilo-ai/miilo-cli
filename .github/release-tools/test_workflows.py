"""Publication graph invariants: missing credentials or failed qualification cannot publish."""
from pathlib import Path
import re
import unittest
import yaml

ROOT = Path(__file__).resolve().parents[1]


def workflow(name):
    return yaml.safe_load((ROOT/'workflows'/name).read_text())


class WorkflowTests(unittest.TestCase):
    def test_release_publication_is_downstream_of_qualification(self):
        jobs = workflow('release.yml')['jobs']
        self.assertIn('qualify', jobs['publication-gate']['needs'])
        self.assertIn('publication-gate', jobs.get('plan', jobs.get('build', jobs.get('github-release')))['needs'])
        for name in ['plan', 'build', 'github-release']:
            if name in jobs:
                self.assertIn("outputs.publish == 'true'", jobs[name]['if'])
        self.assertIn('No release has been made', str(jobs['no-release']))

    def test_main_version_bump_authorizes_release_by_default(self):
        jobs = workflow('version.yml')['jobs']
        policy = next(s for s in jobs['candidate']['steps'] if s.get('id') == 'policy')
        expression = policy['env']['MIILO_PUBLISH']
        self.assertIn("github.event_name == 'push'", expression)
        self.assertIn("github.ref == 'refs/heads/main'", expression)
        self.assertIn("vars.MIILO_RELEASE_ENABLED != 'false'", expression)
        self.assertIn("inputs.publish == true", expression)
        self.assertIn("outputs.new == 'true'", jobs['qualify']['if'])

    def test_auto_tag_never_overwrites_and_dispatches_explicitly(self):
        jobs = workflow('version.yml')['jobs']
        self.assertIn('qualify', jobs['tag']['needs'])
        self.assertIn("outputs.ready == 'true'", jobs['tag']['if'])
        script = jobs['tag']['steps'][-1]['run']
        self.assertIn('gh workflow run release.yml', script)
        self.assertNotIn('--force', script)
        self.assertNotIn('git tag -f', script)

    def test_actions_are_immutable_and_no_target_pr_execution(self):
        for file in (ROOT/'workflows').glob('*.yml'):
            data = yaml.safe_load(file.read_text())
            events = data.get('on', data.get(True, {}))
            self.assertNotIn('pull_request_target', events, str(file))
            for job in data['jobs'].values():
                for step in job.get('steps', []):
                    uses = step.get('uses', '')
                    if '@' in uses:
                        self.assertRegex(uses.rsplit('@', 1)[1], r'^[a-f0-9]{40}$')

    def test_homebrew_missing_token_fails_before_checkout(self):
        jobs = workflow('release.yml')['jobs']
        if 'publish-homebrew-formula' not in jobs:return
        steps = workflow('publish-homebrew.yml')['jobs']['publish']['steps']
        self.assertEqual(steps[0]['id'], 'credentials')
        checkout = next(s for s in steps if s.get('uses', '').startswith('actions/checkout@'))
        self.assertEqual(checkout['with']['ref'], 'main')
        self.assertEqual(checkout['with']['fetch-depth'], 0)
        self.assertIn('HOMEBREW_TAP_TOKEN is missing', steps[0]['run'])
        self.assertIn('exit 1', steps[0]['run'])
        self.assertNotIn('No release has been made', steps[0]['run'])
        for step in steps[1:]:
            gate = step['if']
            self.assertTrue("outputs.ready == 'true'" in gate or "steps.prepare.outputs.changed == 'true'" in gate)

    def test_homebrew_uses_review_branch_and_same_scoped_pr_credential(self):
        data = workflow('publish-homebrew.yml')
        self.assertIn('workflow_dispatch', data['on'])
        self.assertEqual(data['permissions'], {'contents': 'read'})
        job = data['jobs']['publish']
        self.assertEqual(job['env']['GH_TOKEN'], '${{ secrets.HOMEBREW_TAP_TOKEN }}')
        script = '\n'.join(step.get('run', '') for step in job['steps'])
        self.assertNotIn('git push', script)
        self.assertIn('--preflight', script)
        self.assertIn('--prepare', script)
        self.assertIn('Open or reuse formula update PR', str(job['steps']))
        self.assertNotIn('HEAD:main', script)
        downloads = [step for step in job['steps'] if step.get('name', '').startswith('Fetch')]
        self.assertEqual(len(downloads), 2)
        self.assertIn("inputs.plan != ''", downloads[0]['if'])
        self.assertIn("inputs.plan == ''", downloads[1]['if'])



if __name__ == '__main__':unittest.main()
