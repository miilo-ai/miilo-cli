import contextlib
import io
import os
from pathlib import Path
import subprocess
import tempfile
import urllib.error
import unittest
from unittest.mock import patch
import publication
import release_exists
import sdk_access
import version


class PolicyTests(unittest.TestCase):
    def test_semver_precedence_and_invalid_input(self):
        self.assertLess(version.version_key('1.0.0-rc.2'), version.version_key('1.0.0-rc.10'))
        self.assertLess(version.version_key('1.0.0-rc.10'), version.version_key('1.0.0'))
        self.assertEqual(version.version_key('1.0.0+one'), version.version_key('1.0.0+two'))
        for bad in ['1.2', '01.2.3', '1.2.3-01', 'v1.2.3', '1.2.3\nwrite=bad']:
            with self.assertRaises(ValueError): version.version_key(bad)

    def test_version_advance_existing_tag_and_downgrade(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            root.joinpath('Cargo.toml').write_text('[workspace.package]\nversion="1.2.3"\n')
            self.assertEqual(version.candidate(root, ['v1.2.2'])['new_version'], 'true')
            self.assertEqual(version.candidate(root, ['v1.2.3'])['new_version'], 'false')
            with self.assertRaises(ValueError): version.candidate(root, ['v2.0.0'])
            with self.assertRaises(ValueError): version.candidate(root, [], 'v9.0.0')

    def test_sdk_manifest_drift_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ['python', 'rust', 'typescript']: root.joinpath(name).mkdir()
            root.joinpath('python/pyproject.toml').write_text('[project]\nversion="1.0.0"')
            root.joinpath('rust/Cargo.toml').write_text('[package]\nversion="1.0.1"')
            root.joinpath('typescript/package.json').write_text('{"version":"1.0.0"}')
            with self.assertRaises(ValueError): version.read_version(root)

    def test_publication_requires_intent_token_and_dependencies(self):
        valid = dict(MIILO_PUBLISH='true', RELEASE_TOKEN='private-test-token')
        self.assertTrue(publication.permitted(valid))
        for change in [dict(RELEASE_TOKEN=''), dict(DEPENDENCIES_READY='false'), dict(GITHUB_EVENT_NAME='pull_request')]:
            self.assertFalse(publication.permitted(valid | change))
        self.assertFalse(publication.permitted(dict(RELEASE_TOKEN='private-test-token')))

    def test_missing_token_receipt_is_exact_and_secret_free(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)/'output'; summary = Path(directory)/'summary'
            with patch.dict(os.environ, {'GITHUB_OUTPUT': str(output), 'GITHUB_STEP_SUMMARY': str(summary)}, clear=True):
                with contextlib.redirect_stdout(io.StringIO()) as captured: publication.main()
            self.assertEqual(captured.getvalue().strip(), 'No release has been made')
            self.assertIn('publish=false', output.read_text())
            self.assertEqual(summary.read_text().strip(), 'No release has been made')

    def test_private_repository_tagging_has_no_public_visibility_requirement(self):
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['git', 'init', '-q', directory], check=True)
            root = Path(directory)
            root.joinpath('Cargo.toml').write_text('[workspace.package]\nversion="0.1.0"')
            subprocess.run(['git', '-C', directory, 'add', '.'], check=True)
            subprocess.run(['git', '-C', directory, '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'fixture'], check=True)
            subprocess.run(['git', '-C', directory, 'tag', 'v0.1.0'], check=True)
            self.assertEqual(version.candidate(root, ['v0.1.0'])['new_version'], 'false')

    def test_sdk_commit_pins_are_exact_unambiguous_and_canonical(self):
        base={'git':'https://github.com/miilo-ai/miilo-sdk'}
        self.assertEqual(sdk_access.pinned_reference(base|{'rev':'a'*40}),'a'*40)
        for change in [{},{'rev':'main'},{'rev':'a'*40,'tag':'v0.1.0'},{'tag':'main'},{'git':'https://example.invalid/sdk','rev':'a'*40}]:
            with self.assertRaises(ValueError):sdk_access.pinned_reference(base|change)

    def test_private_sdk_without_cross_repo_token_skips(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)/'output'
            dep = {'workspace': {'dependencies': {'miilo-adapter-sdk': {'git': 'https://github.com/miilo-ai/miilo-sdk', 'tag': 'v0.1.0'}}}}
            error = urllib.error.HTTPError('https://api.github.com', 404, 'not found', {}, None)
            with patch.dict(os.environ, {'GITHUB_OUTPUT': str(output)}, clear=True), patch('sdk_access.Path.read_text', return_value=''), patch('sdk_access.tomllib.loads', return_value=dep), patch('sdk_access.urllib.request.urlopen', side_effect=error):
                with contextlib.redirect_stdout(io.StringIO()) as captured: sdk_access.main()
            self.assertIn('No release has been made', captured.getvalue())
            self.assertIn('ready=false', output.read_text())

    def test_configured_invalid_sdk_token_fails_without_echo(self):
        dep = {'workspace': {'dependencies': {'miilo-adapter-sdk': {'git': 'https://github.com/miilo-ai/miilo-sdk', 'tag': 'v0.1.0'}}}}
        error = urllib.error.HTTPError('https://api.github.com', 401, 'bad token', {}, None)
        with patch.dict(os.environ, {'SDK_READ_TOKEN': 'sensitive-test-only'}, clear=True), patch('sdk_access.Path.read_text', return_value=''), patch('sdk_access.tomllib.loads', return_value=dep), patch('sdk_access.urllib.request.urlopen', side_effect=error):
            with self.assertRaises(RuntimeError) as raised: sdk_access.main()
        self.assertNotIn('sensitive-test-only', str(raised.exception))

    def test_existing_release_is_a_noop_and_auth_failure_is_not_hidden(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'output'
            env={'GITHUB_OUTPUT':str(output),'GITHUB_REPOSITORY':'example/private','RELEASE_TAG':'v1.0.0','RELEASE_TOKEN':'test-secret'}
            with patch.dict(os.environ,env,clear=True), patch('release_exists.urllib.request.urlopen',return_value=io.BytesIO(b'{}')):
                release_exists.main()
            self.assertIn('exists=true',output.read_text())
            error=urllib.error.HTTPError('https://api.github.com',401,'denied',{},None)
            with patch.dict(os.environ,env,clear=True), patch('release_exists.urllib.request.urlopen',side_effect=error):
                with self.assertRaises(RuntimeError) as raised:release_exists.main()
            self.assertNotIn('test-secret',str(raised.exception))


if __name__ == '__main__': unittest.main()
