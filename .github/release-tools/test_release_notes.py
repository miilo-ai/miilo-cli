import unittest
from release_notes import render

class CustomTapInstructions(unittest.TestCase):
    def test_custom_repository_has_explicit_registration_and_stable_alias(self):
        output=render('```sh\nbrew install miilo-ai/miilo-homebrew-tap/miilo\n```')
        self.assertIn('brew tap miilo-ai/tap https://github.com/miilo-ai/miilo-homebrew-tap.git',output)
        self.assertIn('brew install miilo-ai/tap/miilo',output)
        self.assertEqual(render(output),output)
        self.assertEqual(render('No Homebrew section.'),'No Homebrew section.')
