#!/usr/bin/env python3
"""Execute the current Python oracle controls with individual observed results."""
import argparse
import json
from pathlib import Path
import platform
import sys
import unittest

from report_issued_test_results import ROOT, file_sha256


def flatten(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from flatten(test)
        else:
            yield test


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.observed = {}
        self.current = {}

    def startTest(self, test):
        super().startTest(test)
        if test.id() in self.current or test.id() in self.observed:
            raise ValueError('duplicate Python control: '+test.id())
        self.current[test.id()] = []

    def addSuccess(self, test):
        super().addSuccess(test)
        self.current[test.id()].append('passed')

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self.current[test.id()].append('failed')

    def addError(self, test, err):
        super().addError(test, err)
        self.current[test.id()].append('not-run')

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        # A skipped subtest is also an unresolved part of its enclosing control.
        identity = getattr(test, 'test_case', test).id()
        self.current[identity].append('not-run')

    def addExpectedFailure(self, test, err):
        super().addExpectedFailure(test, err)
        self.current[test.id()].append('not-run')

    def addUnexpectedSuccess(self, test):
        super().addUnexpectedSuccess(test)
        self.current[test.id()].append('failed')

    def addSubTest(self, test, subtest, err):
        super().addSubTest(test, subtest, err)
        if err is not None:
            self.current[test.id()].append('failed' if issubclass(err[0],test.failureException) else 'not-run')

    def stopTest(self, test):
        outcomes = set(self.current.pop(test.id()))
        self.observed[test.id()] = ('failed' if 'failed' in outcomes else 'passed'
                                   if outcomes == {'passed'} else 'not-run')
        super().stopTest(test)


def validate(record):
    inventory=record.get('inventory',[])
    results=record.get('results',{})
    if (not inventory or len(set(inventory)) != len(inventory)
            or set(inventory) != set(results) or record.get('count') != len(inventory)
            or not all(v in ('passed','failed','not-run') for v in results.values())):
        raise ValueError('incomplete Python control observations')
    return results


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    suite=unittest.defaultTestLoader.discover(str(ROOT/'scripts/ci'),pattern='test_*.py')
    inventory=[test.id() for test in flatten(suite)]
    result=unittest.TextTestRunner(verbosity=2,resultclass=RecordedResult).run(suite)
    record={'schema_version':1,'kind':'python-oracle-controls','inventory':inventory,
            'results':result.observed,'count':result.testsRun,
            'interpreter':{'path':sys.executable,'sha256':file_sha256(Path(sys.executable)),
                           'version':platform.python_version()}}
    args.out.write_text(json.dumps(record,indent=2)+'\n')
    results=validate(record)
    return 0 if result.wasSuccessful() and set(results.values())=={'passed'} else 1


if __name__=='__main__':
    raise SystemExit(main())
