import os
from pathlib import Path
import tempfile
import unittest
from archive_paths import validate_zip_members


class ArchiveDestinations(unittest.TestCase):
    def test_portable_relative_members(self):
        with tempfile.TemporaryDirectory() as directory:
            validate_zip_members(Path(directory), ['miilo/miilo.exe', 'miilo/LICENSE', 'miilo/'])

    def test_traversal_absolute_and_windows_drive_members_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ['../escape', '/escape', 'dir/../../escape', '..\\escape', 'C:\\escape', 'C:escape', '\\\\server\\share\\escape']:
                with self.subTest(name=name), self.assertRaises(RuntimeError):
                    validate_zip_members(directory, [name])

    @unittest.skipIf(os.name == 'nt', 'Creating symlinks requires Windows privileges')
    def test_aliased_extraction_root_is_canonicalized(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'actual'
            root.mkdir()
            alias = Path(directory) / 'alias'
            alias.symlink_to(root, target_is_directory=True)
            validate_zip_members(alias, ['miilo.exe'])
            (root / 'outside').symlink_to(Path(directory), target_is_directory=True)
            with self.assertRaises(RuntimeError):
                validate_zip_members(alias, ['outside/escape'])
