import unittest

from app.runtime_policy import (
    PROVIDER_MOCK,
    PROVIDER_OPENAI,
    normalize_provider,
    parse_env_bool,
    should_use_openai,
)


class RuntimePolicyTests(unittest.TestCase):
    def test_parse_env_bool_should_respect_defaults_and_literals(self) -> None:
        self.assertTrue(parse_env_bool("true"))
        self.assertTrue(parse_env_bool("1"))
        self.assertFalse(parse_env_bool("false"))
        self.assertFalse(parse_env_bool("0"))
        self.assertTrue(parse_env_bool("unexpected", default=True))
        self.assertFalse(parse_env_bool(None, default=False))

    def test_normalize_provider_should_fallback_to_mock(self) -> None:
        self.assertEqual(normalize_provider("openai"), PROVIDER_OPENAI)
        self.assertEqual(normalize_provider("OPENAI"), PROVIDER_OPENAI)
        self.assertEqual(normalize_provider("invalid"), PROVIDER_MOCK)
        self.assertEqual(normalize_provider(None), PROVIDER_MOCK)

    def test_should_use_openai_requires_provider_and_api_key(self) -> None:
        self.assertTrue(should_use_openai(PROVIDER_OPENAI, "sk-xx"))
        self.assertFalse(should_use_openai(PROVIDER_OPENAI, ""))
        self.assertFalse(should_use_openai(PROVIDER_MOCK, "sk-xx"))


if __name__ == "__main__":
    unittest.main()
