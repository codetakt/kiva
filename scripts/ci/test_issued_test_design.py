"""Controls for obligation traceability and definition identity, not approval."""
import copy
import unittest
from issued_test_design import review_binding, obligations


class DefinitionBindingTests(unittest.TestCase):
    def setUp(self):
        self.rule = dict(id='IP-x', actor='LRS', when='Existing resource', requirement='No mutation', strength='shall-not', exceptions=['Explicit voidedStatementId request.'], issues=['OPEN-X'], dependencies=[])
        self.design = dict(http_cases=['case'], procedures=['procedure'], exception_branches=[dict(condition='Explicit voidedStatementId request.', procedure='procedure')])
        self.cases = {'case': dict(sha256='original', assertions=[dict(pointer='/expect/status', sha256='status')])}
        self.procedures = {'procedure': dict(fixture='known ids', steps=['query explicit voided id'], assertions=['return that id'])}
        self.binding = review_binding(self.rule, self.design, self.cases, self.procedures)

    def test_identity_is_independent_of_object_key_order(self):
        rule = dict(reversed(list(self.rule.items())))
        self.assertEqual(self.binding, review_binding(rule, self.design, self.cases, self.procedures))

    def test_actor_condition_strength_or_exception_changes_identity(self):
        for field, value in [('actor', 'LRP'), ('when', 'Any request'), ('strength', 'should'), ('exceptions', [])]:
            changed = dict(self.rule, **{field: value})
            with self.subTest(field=field):
                self.assertNotEqual(self.binding, review_binding(changed, self.design, self.cases, self.procedures))

    def test_assertion_request_procedure_or_exception_changes_identity(self):
        for part in ['assertion', 'request', 'procedure', 'exception']:
            cases = copy.deepcopy(self.cases)
            procedures = copy.deepcopy(self.procedures)
            design = copy.deepcopy(self.design)
            if part == 'assertion': cases['case']['assertions'][0]['sha256'] = 'weakened'
            elif part == 'request': cases['case']['sha256'] = 'different-precondition'
            elif part == 'procedure': procedures['procedure']['assertions'] = []
            else: design['exception_branches'] = []
            with self.subTest(part=part):
                self.assertNotEqual(self.binding, review_binding(self.rule, design, cases, procedures))

    def test_unknown_exception_cannot_silently_inherit_generic_case(self):
        rule = dict(self.rule, exceptions=['New unreviewed exception'], issues=[])
        with self.assertRaisesRegex(ValueError, 'exception lacks an explicit test'):
            obligations(rule, 'authored-duty-correspondence', {'case'}, set(), {}, [])

    def test_known_exception_requires_available_procedure(self):
        rule = dict(self.rule, issues=[])
        with self.assertRaisesRegex(ValueError, 'exception lacks an explicit test'):
            obligations(rule, 'authored-duty-correspondence', {'case'}, set(), {}, [])
        name = 'issued-reference-graph-and-pages'
        result = obligations(rule, 'authored-duty-correspondence', {'case'}, set(), {name: {}}, [])
        self.assertEqual(result['exception_branches'][0]['procedure'], name)
        self.assertIn(name, result['procedures'])

    def test_interpretation_issue_requires_concrete_procedure(self):
        rule = dict(self.rule, exceptions=[])
        with self.assertRaisesRegex(ValueError, 'missing issue test'):
            obligations(rule, 'authored-duty-correspondence', {'case'}, set(), {}, [])

    def test_recommendation_and_producer_role_retain_their_strength(self):
        rule = dict(self.rule, actor='LRP', strength='should', exceptions=[], issues=[])
        result = obligations(rule, 'authored-duty-correspondence', {'case'}, {'procedure'}, self.procedures, [])
        self.assertEqual(result['effect']['grading'], 'advisory')
        self.assertEqual(result['applicability']['role'], 'role-specific')
        self.assertEqual(result['applicability']['condition'], rule['when'])
        with self.assertRaisesRegex(ValueError, 'role-specific rule lacks role procedure'):
            obligations(rule, 'authored-duty-correspondence', {'case'}, set(), {}, [])


if __name__ == '__main__': unittest.main()
