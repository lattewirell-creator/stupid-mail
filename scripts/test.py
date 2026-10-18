"""Run deterministic checks without real mail accounts, tokens, or messages."""
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[1]
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=root, check=True)
with tempfile.TemporaryDirectory(prefix='stupid-mail-tests-') as temporary:
    temp = Path(temporary)
    for test, sources in [
        ('CoreTests', ['MailCore', 'IMAPConnection', 'MailCache']),
        ('ProviderTests', ['MailCore', 'IMAPConnection', 'OutlookMail']),
        ('WorkspaceTests', ['MailCore', 'IMAPConnection', 'WorkspaceCore']),
        ('CalendarTests', ['CalendarCore']),
    ]:
        binary = temp / test
        run('swiftc', *[root / 'macos' / (name + '.swift') for name in sources],
            root / 'tests' / (test + '.swift'), '-o', binary,
            '-framework', 'Security', '-framework', 'Network',
            '-module-cache-path', Path(tempfile.gettempdir()) / 'stupid-mail-test-module-cache')
        run(binary)
for test in ['logic.test.cjs', 'app-flows.test.cjs', 'buddy-flows.test.cjs', 'music.test.cjs', 'buddy-play.test.cjs', 'customize.test.cjs']:
    run('node', root / 'tests' / test)
