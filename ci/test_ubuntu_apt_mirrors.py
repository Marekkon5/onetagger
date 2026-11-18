"""Check the Ubuntu mirror ordering and archive source selection."""
import unittest

from ubuntu_apt_mirrors import MIRROR_LIST, archive_settings, mirror_list


class AptMirrorTests(unittest.TestCase):
    def test_fastest_mirror_first_and_original_retained_when_probe_fails(self):
        default = "http://us-east-2.ec2.archive.ubuntu.com/ubuntu"
        speeds = {
            default: 0,
            "https://archive.ubuntu.com/ubuntu": 200_000,
            "https://mirrors.edge.kernel.org/ubuntu": 900_000,
            "https://mirror.math.princeton.edu/pub/ubuntu": 0,
        }
        self.assertEqual(mirror_list(default, speeds), (
            "https://mirrors.edge.kernel.org/ubuntu/\tpriority:1\n"
            "https://archive.ubuntu.com/ubuntu/\tpriority:2\n"
            "http://us-east-2.ec2.archive.ubuntu.com/ubuntu/\tpriority:3\n"
        ))

    def test_rewrites_archive_and_keeps_security_on_the_same_host(self):
        uri = "http://azure.archive.ubuntu.com/ubuntu/"
        key = "Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg"
        archive = f"Types: deb\nURIs: {uri}\nSuites: noble noble-updates noble-backports\n{key}\n"
        security = f"Types: deb\nURIs: {uri}\nSuites: noble-security\n{key}\n"
        other = "Types: deb\nURIs: https://packages.microsoft.com/ubuntu/24.04/prod\nSuites: noble\n"
        original, updated = archive_settings(archive + "\n" + security + "\n" + other)
        expected = archive.replace(uri, f"mirror+file:{MIRROR_LIST}") + "\n" + security + "\n" + other
        self.assertEqual(original, uri.rstrip("/"))
        self.assertEqual(updated, expected)


if __name__ == "__main__":
    unittest.main()
