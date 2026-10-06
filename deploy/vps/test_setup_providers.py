import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch

import setup_providers as setup


class ProviderSetupTests(unittest.TestCase):
    def test_replacement_preserves_other_settings_and_removes_duplicate_secret_keys(self):
        original = '# keep this comment\nCORS_ORIGIN=https://mixsets.syco23.org\nDISCOGS_TOKEN=old\nDISCOGS_TOKEN=older\n'
        updated = setup.replace_values(original, {'DISCOGS_TOKEN': 'new'})
        self.assertIn('# keep this comment', updated)
        self.assertIn('CORS_ORIGIN=https://mixsets.syco23.org', updated)
        self.assertEqual(updated.count('DISCOGS_TOKEN='), 1)
        self.assertIn('DISCOGS_TOKEN=new', updated)

    def test_secret_file_is_private_after_atomic_update(self):
        with tempfile.TemporaryDirectory() as directory:
            env_file = Path(directory) / '.env'
            env_file.write_text('CORS_ORIGIN=https://mixsets.syco23.org\n')
            os.chmod(env_file, 0o644)
            setup.save_env(env_file, {'SOUNDCLOUD_CLIENT_ID': 'test-id', 'SOUNDCLOUD_CLIENT_SECRET': 'test-secret'})
            self.assertEqual(stat.S_IMODE(env_file.stat().st_mode), 0o600)
            self.assertIn('CORS_ORIGIN=https://mixsets.syco23.org', env_file.read_text())
            self.assertIn('SOUNDCLOUD_CLIENT_SECRET=test-secret', env_file.read_text())

    def test_soundcloud_verifies_token_and_public_search_before_saving(self):
        with patch.object(setup, 'request_json', side_effect=[{'access_token': 'test-token'}, {'collection': []}]) as request:
            setup.validate_soundcloud('test-id', 'test-secret')
            self.assertEqual(request.call_count, 2)
            self.assertTrue(request.call_args_list[0].args[1]['Authorization'].startswith('Basic '))
            self.assertEqual(request.call_args_list[1].args[1]['Authorization'], 'OAuth test-token')

    def test_unsafe_dotenv_characters_are_rejected(self):
        for value in ('', 'bad value', 'bad#value', 'bad$value', 'bad\nvalue'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                setup.safe_value(value)


if __name__ == '__main__':
    unittest.main()
