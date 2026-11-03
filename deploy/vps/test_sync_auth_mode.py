import tempfile
import unittest
from pathlib import Path
from sync_auth_mode import sync


class AuthModeSyncTests(unittest.TestCase):
    def test_copies_only_mode_preserves_private_provider_secrets_and_permissions(self):
        with tempfile.TemporaryDirectory() as directory:
            source, target = Path(directory) / "vercel.env", Path(directory) / ".env"
            source.write_text('AUTH_MODE="off"\nIGNORED_SECRET=do-not-copy\n')
            target.write_text("DISCOGS_TOKEN=private-existing-value\nAUTH_MODE=on\nAUTH_MODE=on\n")
            self.assertEqual(sync(source, target), "off")
            self.assertEqual(target.read_text(), "DISCOGS_TOKEN=private-existing-value\nAUTH_MODE=off\n")
            self.assertEqual(target.stat().st_mode & 0o777, 0o600)

    def test_invalid_missing_duplicate_or_unconfigured_on_keeps_target_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            source, target = Path(directory) / "vercel.env", Path(directory) / ".env"
            target.write_text("AUTH_MODE=off\n")
            for contents in ("", "AUTH_MODE=false\n", "AUTH_MODE=off\nAUTH_MODE=on\n", "AUTH_MODE=on\n"):
                source.write_text(contents)
                with self.assertRaises(ValueError):
                    sync(source, target)
                self.assertEqual(target.read_text(), "AUTH_MODE=off\n")


if __name__ == "__main__":
    unittest.main()
