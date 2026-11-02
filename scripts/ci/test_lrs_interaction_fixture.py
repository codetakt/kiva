"""Independent paper examples for the separately declared consumer fixture."""
import unittest
from lrs_interaction_fixture import correct,pattern_matches,parameters,string_item


class InteractionFixtureTests(unittest.TestCase):
    def test_complete_patterns_all_types_and_component_cardinality(self):
        for kind,pattern,response,wrong in (
            ('true-false','true','true','false'),('choice','a[,]b','b[,]a','a'),
            ('sequencing','b[,]a','b[,]a','a[,]b'),('likert','a','a','b'),
            # s2 has zero associations; s1 has two, with separate targets.
            ('matching','s1[.]t1[,]s1[.]t2','s1[.]t2[,]s1[.]t1','s1[.]t1'),
            ('performance','p1[.]text[,]p2[.]1[:]2','p1[.]text[,]p2[.]1.5','p1[.]text[,]p2[.]3'),
            ('numeric','1','1.0','2'),('numeric','1[:]2','1.5','3'),
            ('numeric','[:]2','-17','3'),('numeric','1[:]','17','0'),
            ('fill-in','Alpha[,]Beta','alpha[,]BETA','alpha'),
            ('long-fill-in','Alpha[,]Beta','alpha[,]BETA','alpha'),
            ('other','any string','any string','another string'),
        ):
            with self.subTest(kind=kind,pattern=pattern):
                self.assertTrue(pattern_matches(kind,pattern,response))
                self.assertFalse(pattern_matches(kind,pattern,wrong))
        self.assertTrue(correct('choice',['a','b'],'b'))
        self.assertFalse(correct('choice',['a','b'],'a[,]b'))
        self.assertTrue(pattern_matches('fill-in','学'*251,'学'*251))
        malformed={'true-false':'TRUE','choice':'a[,]','sequencing':'b[.]a','likert':'a[,]b',
                   'matching':'s1[.]','performance':'p1text[,]p2[.]1[:]2','numeric':'1[:]2[:]3',
                   'fill-in':'a[,]','long-fill-in':'a[,]','other':17}
        for kind,pattern in malformed.items():
            # Nonempty-item restriction is this finite interpreter's policy;
            # it never becomes an LRS rejection expectation.
            with self.subTest(kind=kind),self.assertRaises(ValueError):
                pattern_matches(kind,pattern,'a')

    def test_characterstring_defaults_positions_and_type_boundaries(self):
        for kind in ('fill-in','long-fill-in'):
            for prefix in ('','{case_matters=false}','{case_matters=false}{order_matters=true}'):
                self.assertTrue(pattern_matches(kind,prefix+'Alpha[,]Beta','alpha[,]BETA'))
                self.assertFalse(pattern_matches(kind,prefix+'Alpha[,]Beta','beta[,]alpha'))
            self.assertFalse(pattern_matches(kind,'{case_matters=true}Alpha[,]Beta','alpha[,]Beta'))
            self.assertTrue(pattern_matches(kind,'{case_matters=true}Alpha[,]Beta','Alpha[,]Beta'))
            self.assertTrue(pattern_matches(kind,'{order_matters=false}Alpha[,]Beta','beta[,]alpha'))
            tagged='{lang=ja}学習[,]{lang=en}Study'
            self.assertTrue(pattern_matches(kind,tagged,'{lang=ja}学習[,]{lang=en}study'))
            self.assertFalse(pattern_matches(kind,tagged,'{lang=en}学習[,]{lang=en}Study'))
            self.assertEqual(string_item('{lang=ja}学習',False)[:2],('学習','ja'))
            self.assertEqual(string_item('{lang=en}Study',False)[:2],('study','en'))
            # A list parameter at a later item has no list-scope effect.
            self.assertFalse(pattern_matches(kind,'Alpha[,]{case_matters=true}Beta','alpha[,]Beta'))
            for prefix in ('{case_matters=invalid}','{unknown=invalid}'):
                self.assertTrue(pattern_matches(kind,prefix+'Alpha','alpha'))
        base='p1[.]{lang=ja}学習[,]p2[.]{lang=en}Study'
        for prefix in ('','{order_matters=true}'):
            self.assertTrue(pattern_matches('performance',prefix+base,base))
            self.assertFalse(pattern_matches('performance',prefix+base,'p2[.]{lang=en}Study[,]p1[.]{lang=ja}学習'))
        self.assertTrue(pattern_matches('performance','{order_matters=false}'+base,'p2[.]{lang=en}Study[,]p1[.]{lang=ja}学習'))
        self.assertFalse(pattern_matches('performance','{case_matters=false}p1[.]Study','p1[.]study'))
        self.assertTrue(pattern_matches('performance','p1[.]1[:]2','p1[.]1.5'))
        with self.assertRaises(ValueError):pattern_matches('performance','p1[.]{lang=en}1[:]2','p1[.]1.5')
        self.assertEqual(parameters('{unknown=a}{case_matters=invalid}Text',{'case_matters'}),
                         ('Text',{},[('unknown','a'),('case_matters','invalid')]))


if __name__=='__main__':unittest.main()
