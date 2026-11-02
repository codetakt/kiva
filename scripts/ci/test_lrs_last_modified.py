"""IEEE empty-set semantics and the separate selected Kiva D-02=A boundary."""
import unittest
from run_lrs_procedures import Executor


class LastModifiedTests(unittest.TestCase):
    def check(self, header, items, **capabilities):
        return Executor.page_last_modified(None,{} if header is None else {'last-modified':header},items,**capabilities)

    def test_empty_needs_representation_and_origination_evidence(self):
        header='Tue, 20 Oct 2026 00:00:00 GMT'
        bounds={'representation_modified':'2026-10-20T00:00:00.999Z','originated':'2026-10-20T00:00:02Z'}
        self.assertIs(self.check(None,[]),True)
        self.assertIs(self.check(header,[],**bounds),True)
        self.assertIsNone(self.check(header,[]))
        for key in bounds:
            self.assertIsNone(self.check(header,[],**{key:bounds[key]}))
        self.assertIs(self.check('invalid',[]),False)
        self.assertIs(self.check('Tue, 20 Oct 2026 00:00:03 GMT',[],**bounds),False)
        self.assertIs(self.check('Tue, 20 Oct 2026 00:00:03 GMT',[],originated=bounds['originated']),False)
        self.assertIs(self.check('Mon, 19 Oct 2026 23:59:59 GMT',[],**bounds),False)
        # This other-LRS response satisfies the IEEE relation but differs from
        # Kiva's independently selected omission policy. Presence is not FAIL.
        self.assertIs(self.check(header,[],**bounds),True)
        kiva_omission_matches=header is None
        self.assertFalse(kiva_omission_matches)

    def test_nonempty_maximum_rounding_and_missing_provider(self):
        header='Tue, 20 Oct 2026 00:00:00 GMT'
        low={'stored':'2026-10-19T23:59:59.999Z'}
        high={'stored':'2026-10-20T00:00:00.999Z'}
        tie={'stored':'2026-10-20T00:00:00.125Z'}
        for items in ([high],[low,high,tie],[tie,high,low],[high,tie,low]):
            self.assertIs(self.check(header,items),True)
            self.assertIs(self.check(None,items),False)
            self.assertIs(self.check('Tue, 20 Oct 2026 00:00:01 GMT',items),False)
        outside={'stored':'2016-12-31T23:59:60Z'}
        self.assertIs(self.check(None,[outside]),False)
        self.assertIs(self.check('invalid',[outside]),False)
        self.assertIsNone(self.check(header,[outside]))
        self.assertIs(self.check(header,[outside,{'stored':'2026-10-20T00:00:01Z'}]),False)


if __name__=='__main__':unittest.main()
