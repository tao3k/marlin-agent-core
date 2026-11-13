# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

import unittest

from run_gerbil_property_gate import module_succeeded


class GerbilReceiptTest(unittest.TestCase):
    def test_requires_module_success(self):
        self.assertTrue(module_succeeded("MODULE t/all-test.ss\nMODULE-OK t/all-test.ss\nHARNESS-OK\n"))
        self.assertFalse(module_succeeded("MODULE t/all-test.ss\nHARNESS-OK\nERROR MODULE t/all-test.ss\n"))
        self.assertFalse(module_succeeded("MODULE-OK t/all-test.ss\nERROR MODULE t/other.ss\n"))
        self.assertFalse(module_succeeded("HARNESS-OK\n"))
        self.assertTrue(module_succeeded("MODULE-OK t/ui-native-test.ss\n", "t/ui-native-test.ss"))
        self.assertFalse(module_succeeded("MODULE-OK t/ui-native-test.ss\nERROR CASE failed\n", "t/ui-native-test.ss"))


if __name__ == "__main__":
    unittest.main()
