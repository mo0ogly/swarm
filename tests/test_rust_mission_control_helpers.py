"""Fixture metadata readback retains original read and cleanup failures."""
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock,patch
import rust_mission_report_relay as relay

class ControlReadGuards(unittest.TestCase):
    def test_positive_read_before_primary_and_close_fault(self):
        """A valid read precedes simultaneous failures; retain exact cause objects."""
        owned=SimpleNamespace(root=Path('/owned-fixture'))
        db=MagicMock();db.execute.return_value.fetchall.return_value=[('original',)]
        with patch.object(relay.sqlite3,'connect',return_value=db) as connect:
            self.assertEqual(relay.rows(owned,'SELECT original',()),[('original',)])
            self.assertIn('mode=ro',connect.call_args.args[0]);db.close.assert_called_once()
        primary=RuntimeError('read');cleanup=OSError('close')
        db=MagicMock();db.execute.side_effect=primary;db.close.side_effect=cleanup
        with patch.object(relay.sqlite3,'connect',return_value=db):
            with self.assertRaises(BaseExceptionGroup) as seen:relay.rows(owned,'SELECT original',())
        self.assertEqual(seen.exception.exceptions,(primary,cleanup))

    def test_close_failure_never_turns_valid_read_into_success(self):
        """A successful observation with failed close remains a failure."""
        owned=SimpleNamespace(root=Path('/owned-fixture'));db=MagicMock();cleanup=OSError('close')
        db.execute.return_value.fetchall.return_value=[('original',)];db.close.side_effect=cleanup
        with patch.object(relay.sqlite3,'connect',return_value=db):
            with self.assertRaises(OSError) as seen:relay.rows(owned,'SELECT original',())
        self.assertIs(seen.exception,cleanup)

if __name__=='__main__':unittest.main()
