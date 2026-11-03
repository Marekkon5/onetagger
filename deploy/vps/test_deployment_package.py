from pathlib import Path
import fnmatch
import unittest

ROOT = Path(__file__).resolve().parents[2]


class DeploymentPackageTests(unittest.TestCase):
    def test_private_configuration_and_persistent_data_are_excluded_from_context(self):
        ignore = ROOT / '.dockerignore'
        patterns = ignore.read_text().splitlines() if ignore.exists() else []
        for name in ('.env', '.env.local', 'deploy/vps/.env', 'server/data/catalog.sqlite', 'server/data/media/waveform.png', '.git/config', 'node_modules/secret/config.json'):
            with self.subTest(name=name):
                self.assertTrue(any(fnmatch.fnmatch(name, pattern) or name.startswith(pattern.rstrip('/') + '/') for pattern in patterns if pattern and not pattern.startswith(('#', '!'))), name)

    def test_runtime_can_start_without_a_package_manager(self):
        dockerfile = (ROOT / 'server/Dockerfile').read_text()
        runtime = dockerfile.split(' AS runtime', 1)[1]
        self.assertNotIn('RUN pnpm', runtime)
        self.assertIn('node_modules', runtime)
        self.assertIn('USER node', runtime)
        self.assertIn('node:22.23.3', dockerfile)



if __name__ == '__main__':
    unittest.main()
