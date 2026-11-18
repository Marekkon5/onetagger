"""Check the Ubuntu mirror ordering and archive source selection."""
import unittest

from ubuntu_apt_mirrors import ARCHIVE_URI, mirror_list


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

    def test_archive_pattern_does_not_match_security_or_other_repositories(self):
        source = (
            "Types: deb\n"
            "URIs: http://us-east-2.ec2.archive.ubuntu.com/ubuntu/\n"
            "Suites: noble noble-updates noble-backports\n"
            "Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n\n"
            "Types: deb\n"
            "URIs: https://security.ubuntu.com/ubuntu/\n"
            "Suites: noble-security\n"
            "Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n\n"
            "Types: deb\n"
            "URIs: https://packages.microsoft.com/ubuntu/24.04/prod\n"
        )
        self.assertEqual(ARCHIVE_URI.findall(source), [
            "http://us-east-2.ec2.archive.ubuntu.com/ubuntu/",
        ])


if __name__ == "__main__":
    unittest.main()
