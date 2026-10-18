"""Sign a built release and its public Sparkle feed using the local Keychain."""
from pathlib import Path
import json
import plistlib
import subprocess
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parents[1]
config = json.loads((root / 'updates/config.json').read_text())
release = root / ('release-' + config['version'])
archive = release / 'The Stupid Mail — Mac.zip'
info = plistlib.loads((release / 'The Stupid Mail.app/Contents/Info.plist').read_bytes())
assert info['CFBundleShortVersionString'] == config['version']
assert info['CFBundleVersion'] == str(config['build'])
assert info['SUPublicEDKey'] == config['public_key']
bin_dir = root / '.build/sparkle/bin'
account = ['--account', config['keychain_account']]
def output(*args):
    return subprocess.check_output([str(a) for a in args], text=True).strip()
assert output(bin_dir / 'generate_keys', *account, '-p') == config['public_key'], 'Wrong update signing key'
signature = output(bin_dir / 'sign_update', *account, '-p', archive)
subprocess.run([str(bin_dir / 'sign_update'), *account, '--verify', str(archive), signature], check=True)
namespace = 'http://www.andymatuschak.org/xml-namespaces/sparkle'
ET.register_namespace('sparkle', namespace)
feed = ET.Element('rss', version='2.0')
channel = ET.SubElement(feed, 'channel')
ET.SubElement(channel, 'title').text = 'The Stupid Mail updates'
item = ET.SubElement(channel, 'item')
ET.SubElement(item, 'title').text = 'The Stupid Mail ' + config['version']
ET.SubElement(item, f'{{{namespace}}}version').text = str(config['build'])
ET.SubElement(item, f'{{{namespace}}}shortVersionString').text = config['version']
ET.SubElement(item, f'{{{namespace}}}minimumSystemVersion').text = '13.0'
ET.SubElement(item, 'description').text = 'A new version of The Stupid Mail is ready. Your mail settings and saved drafts stay on this Mac.'
ET.SubElement(item, 'enclosure', {
    'url': f'https://github.com/lattewirell-creator/stupid-mail/releases/download/v{config["version"]}/The.Stupid.Mail.Mac.zip',
    'length': str(archive.stat().st_size), 'type': 'application/octet-stream',
    f'{{{namespace}}}edSignature': signature,
})
ET.indent(feed)
path = root / 'updates/appcast.xml'
ET.ElementTree(feed).write(path, encoding='utf-8', xml_declaration=True)
subprocess.run([str(bin_dir / 'sign_update'), *account, str(path)], check=True)
subprocess.run([str(bin_dir / 'sign_update'), *account, '--verify', str(path)], check=True)
print('Signed archive and feed. Upload the ZIP release before publishing updates/appcast.xml.')
