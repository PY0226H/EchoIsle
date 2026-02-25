import unittest

from app.openai_judge import _merge_two_pass, _normalize_eval


class OpenAiJudgeTests(unittest.TestCase):
    def test_normalize_eval_should_clamp_and_derive_scores(self) -> None:
        normalized = _normalize_eval(
            {
                "winner": "invalid",
                "logic_pro": 120,
                "logic_con": 5,
                "evidence_pro": "80",
                "evidence_con": "30",
                "rebuttal_pro": 70.2,
                "rebuttal_con": 66.7,
                "clarity_pro": 50,
                "clarity_con": 40,
                "pro_summary": " pro summary ",
                "con_summary": "",
                "rationale": " reason ",
            }
        )
        self.assertEqual(normalized["logic_pro"], 100)
        self.assertEqual(normalized["evidence_pro"], 80)
        self.assertEqual(normalized["rebuttal_con"], 67)
        self.assertEqual(normalized["winner"], "pro")
        self.assertGreater(normalized["pro_score"], normalized["con_score"])
        self.assertEqual(normalized["con_summary"], "信息不足。")

    def test_merge_two_pass_should_trigger_draw_on_winner_mismatch(self) -> None:
        first = {
            "winner": "pro",
            "logic_pro": 85,
            "logic_con": 60,
            "evidence_pro": 83,
            "evidence_con": 62,
            "rebuttal_pro": 79,
            "rebuttal_con": 58,
            "clarity_pro": 70,
            "clarity_con": 65,
            "pro_score": 80,
            "con_score": 61,
            "pro_summary": "p1",
            "con_summary": "c1",
            "rationale": "first",
        }
        second = {
            "winner": "con",
            "logic_pro": 70,
            "logic_con": 82,
            "evidence_pro": 68,
            "evidence_con": 84,
            "rebuttal_pro": 64,
            "rebuttal_con": 78,
            "clarity_pro": 69,
            "clarity_con": 75,
            "pro_score": 68,
            "con_score": 80,
            "pro_summary": "p2",
            "con_summary": "c2",
            "rationale": "second",
        }

        merged = _merge_two_pass(first, second)
        self.assertEqual(merged["winner"], "draw")
        self.assertTrue(merged["needs_draw_vote"])
        self.assertTrue(merged["rejudge_triggered"])
        self.assertIn("双次评估胜方不一致", merged["rationale"])


if __name__ == "__main__":
    unittest.main()
