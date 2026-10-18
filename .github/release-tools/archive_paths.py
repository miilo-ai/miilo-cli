"""Validate archive destinations against one canonical extraction root."""
from pathlib import Path, PurePosixPath, PureWindowsPath


def validate_zip_members(root, names):
    root = Path(root).resolve()
    for name in names:
        portable = PurePosixPath(name.replace('\\', '/'))
        if portable.is_absolute() or PureWindowsPath(name).drive or '..' in portable.parts:
            raise RuntimeError('Unsafe archive member')
        if not root.joinpath(*portable.parts).resolve().is_relative_to(root):
            raise RuntimeError('Unsafe archive member')
